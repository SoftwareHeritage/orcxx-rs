// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

//! Helpers for the `orcxx_derive` crate.

#![allow(clippy::redundant_closure_call)]

use unsafe_unwrap::UnsafeUnwrap;

use std::convert::TryInto;
use std::iter::Map;
use std::num::TryFromIntError;
use std::slice::IterMut;
use std::str::Utf8Error;

use kind::Kind;
use utils::OrcError;
use vector::{BorrowedColumnVectorBatch, ColumnVectorBatch, StructVectorBatch};

#[derive(Debug, PartialEq)]
pub enum DeserializationError {
    /// Expected to parse a structure from the ORC file, but the given column is of
    /// an incompatible type. Contains the ORC exception whiched occured when casting.
    MismatchedColumnKind(OrcError),
    /// The structure has a field which was not selected when reading the ORC file (or
    /// is missing from the file).
    /// Contains the name of the field.
    MissingField(String),
    /// u64 could not be converted to usize. Contains the original error
    UsizeOverflow(TryFromIntError),
    /// [`Vec::from_vector_batch`](OrcDeserialize::from_vector_batch) was
    /// called on a non-empty [`Vec`]
    NonEmptyVector,
    /// Failed to decode a [`String`] (use [`Vec<u8>`](`Vec`) instead for columns of
    /// `binary` type).
    Utf8Error(Utf8Error),
    /// [`read_from_vector_batch`](OrcDeserialize::read_from_vector_batch) was called
    /// as a method on a non-`Option` type, with a column containing nulls as parameter.
    ///
    /// Contains a human-readable error.
    UnexpectedNull(String),
}

fn check_kind_equals(got_kind: &Kind, expected_kind: &Kind, type_name: &str) -> Result<(), String> {
    if got_kind == expected_kind {
        Ok(())
    } else {
        Err(format!(
            "{} must be decoded from ORC {:?}, not ORC {:?}",
            type_name, expected_kind, got_kind
        ))
    }
}

/// Types which provide a static `check_kind` method to ensure ORC files can be
/// deserialized into them.
pub trait CheckableKind {
    /// Returns whether the type can be deserialized from [`RowReader`](::reader::RowReader)
    /// instances with this [selected_kind](::reader::RowReader::selected_kind).
    ///
    /// This should be called before any method provided by [`OrcDeserialize`],
    /// to get errors early and with a human-readable error message instead of cast errors
    /// or deserialization into incorrect types (eg. if a file has two fields swapped).
    fn check_kind(kind: &Kind) -> Result<(), String>;
}

// Needed because most structs are going to have Option as fields, and code generated by
// orcxx_derive needs to call check_kind on them recursively.
// This avoid needing to dig into the AST to extract the inner type of the Option.
impl<T: CheckableKind> CheckableKind for Option<T> {
    fn check_kind(kind: &Kind) -> Result<(), String> {
        T::check_kind(kind)
    }
}

/// Types which can be read in batch from ORC columns ([`BorrowedColumnVectorBatch`]).
pub trait OrcDeserialize: Sized + Default + CheckableKind {
    /// Reads from a [`BorrowedColumnVectorBatch`] to a structure that behaves like
    /// a rewindable iterator of `&mut Self`.
    ///
    /// Users should call
    /// [`check_kind(row_reader.selected_kind()).unwrap()`](CheckableKind::check_kind)
    /// before calling this function on batches produces by a `row_reader`.
    fn read_from_vector_batch<'a, 'b, T>(
        src: &BorrowedColumnVectorBatch,
        dst: &'b mut T,
    ) -> Result<(), DeserializationError>
    where
        Self: 'a,
        &'b mut T: DeserializationTarget<'a, Item = Self> + 'b;

    /// Reads from a [`BorrowedColumnVectorBatch`] and returns a `Vec<Option<Self>>`
    ///
    /// Users should call
    /// [`check_kind(row_reader.selected_kind()).unwrap()`](CheckableKind::check_kind)
    /// before calling this function on batches produces by a `row_reader`.
    ///
    /// This is a wrapper for
    /// [`read_from_vector_batch`](OrcDeserialize::read_from_vector_batch)
    /// which takes care of allocating a buffer, and returns it.
    fn from_vector_batch(
        vector_batch: &BorrowedColumnVectorBatch,
    ) -> Result<Vec<Self>, DeserializationError> {
        let num_elements = vector_batch.num_elements();
        let num_elements = num_elements
            .try_into()
            .map_err(DeserializationError::UsizeOverflow)?;
        let mut values = Vec::with_capacity(num_elements);
        values.resize_with(num_elements, Default::default);
        Self::read_from_vector_batch(vector_batch, &mut values)?;
        Ok(values)
    }
}

macro_rules! impl_scalar {
    ($ty:ty, $kind:expr, $method:ident) => {
        impl_scalar!($ty, $kind, $method, |s| Ok(s as $ty));
    };
    ($ty:ty, $kind:expr, $method:ident, $cast:expr) => {
        impl CheckableKind for $ty {
            fn check_kind(kind: &Kind) -> Result<(), String> {
                check_kind_equals(kind, &$kind, stringify!($ty))
            }
        }

        impl OrcDeserialize for $ty {
            fn read_from_vector_batch<'a, 'b, T>(
                src: &BorrowedColumnVectorBatch,
                mut dst: &'b mut T,
            ) -> Result<(), DeserializationError>
            where
                &'b mut T: DeserializationTarget<'a, Item = Self> + 'b,
            {
                if src.not_null().is_some() {
                    // If it is `Some`, there is at least one null so we are going to
                    // crash eventually. Exit early to avoid checking every single value
                    // later.
                    return Err(DeserializationError::UnexpectedNull(format!(
                        "{} column contains nulls",
                        stringify!($ty)
                    )));
                }
                let src = src
                    .$method()
                    .map_err(DeserializationError::MismatchedColumnKind)?;
                for (s, d) in src.iter().zip(dst.iter_mut()) {
                    // This is safe because we checked above this column contains no
                    // nulls (`src.not_null().is_some()`), so `s` can't be None.
                    *d = ($cast)(unsafe { s.unsafe_unwrap() })?
                }

                Ok(())
            }
        }

        impl OrcDeserialize for Option<$ty> {
            fn read_from_vector_batch<'a, 'b, T>(
                src: &BorrowedColumnVectorBatch,
                mut dst: &'b mut T,
            ) -> Result<(), DeserializationError>
            where
                &'b mut T: DeserializationTarget<'a, Item = Self> + 'b,
            {
                let src = src
                    .$method()
                    .map_err(DeserializationError::MismatchedColumnKind)?;
                for (s, d) in src.iter().zip(dst.iter_mut()) {
                    match s {
                        None => *d = None,
                        Some(s) => *d = Some(($cast)(s)?),
                    }
                }

                Ok(())
            }
        }
    };
}

impl_scalar!(bool, Kind::Boolean, try_into_longs, |s| Ok(s != 0));
impl_scalar!(i8, Kind::Byte, try_into_longs);
impl_scalar!(i16, Kind::Short, try_into_longs);
impl_scalar!(i32, Kind::Int, try_into_longs);
impl_scalar!(i64, Kind::Long, try_into_longs);
impl_scalar!(f32, Kind::Float, try_into_doubles);
impl_scalar!(f64, Kind::Double, try_into_doubles);
impl_scalar!(String, Kind::String, try_into_strings, |s| {
    std::str::from_utf8(s)
        .map_err(DeserializationError::Utf8Error)
        .map(|s| s.to_string())
});
impl_scalar!(Vec<u8>, Kind::Binary, try_into_strings, |s: &[u8]| Ok(
    s.to_vec()
));

impl<T: CheckableKind> CheckableKind for Vec<T> {
    fn check_kind(kind: &Kind) -> Result<(), String> {
        match kind {
            Kind::List(inner) => T::check_kind(inner),
            _ => Err(format!("Must be a List, not {:?}", kind)),
        }
    }
}

/// Shared initialization code of `impl<I> OrcDeserializeOption for Vec<I>`
/// and impl<I> OrcDeserialize for Vec<I>
macro_rules! init_list_read {
    ($src:expr, $dst: expr) => {{
        let src = $src
            .try_into_lists()
            .map_err(DeserializationError::MismatchedColumnKind)?;

        let num_lists: usize = src
            .num_elements()
            .try_into()
            .map_err(DeserializationError::UsizeOverflow)?;
        let num_elements: usize = src
            .elements()
            .num_elements()
            .try_into()
            .map_err(DeserializationError::UsizeOverflow)?;

        assert_eq!(
            $dst.len(),
            num_lists,
            "dst has length {}, expected {}",
            $dst.len(),
            num_lists
        );

        // Deserialize the inner elements recursively into this temporary buffer.
        // TODO: write them directly to the final location to avoid a copy
        let mut elements = Vec::new();
        elements.resize_with(num_elements, Default::default);
        OrcDeserialize::read_from_vector_batch::<Vec<I>>(&src.elements(), &mut elements)?;

        let elements = elements.into_iter().enumerate();

        let offsets = src.iter_offsets();

        (offsets, elements)
    }};
}

/// Shared loop code of `impl<I> OrcDeserializeOption for Vec<I>`
/// and impl<I> OrcDeserialize for Vec<I>
macro_rules! build_list_item {
    ($range:expr, $last_offset:expr, $elements:expr) => {{
        let range = $range;
        assert_eq!(
            range.start, $last_offset,
            "Non-continuous list (jumped from offset {} to {}",
            $last_offset, range.start
        );
        // Safe because offset is bounded by num_elements;
        let mut array: Vec<I> = Vec::with_capacity((range.end - range.start) as usize);
        loop {
            match $elements.next() {
                Some((i, item)) => {
                    array.push(item);
                    if i == range.end - 1 {
                        break;
                    }
                }
                None => panic!("List too short"),
            }
        }
        $last_offset = range.end;
        array
    }};
}

/// Deserialization of ORC lists with nullable values
///
/// cannot do `impl<I> OrcDeserialize for Option<Vec<Option<I>>>` because it causes
/// infinite recursion in the type-checker due to this other implementation being
/// available: `impl<I: OrcDeserializeOption> OrcDeserialize for Option<I>`.
impl<I> OrcDeserializeOption for Vec<I>
where
    I: Default + OrcDeserialize,
{
    fn read_options_from_vector_batch<'a, 'b, T>(
        src: &BorrowedColumnVectorBatch,
        mut dst: &'b mut T,
    ) -> Result<(), DeserializationError>
    where
        &'b mut T: DeserializationTarget<'a, Item = Option<Self>> + 'b,
    {
        let (offsets, mut elements) = init_list_read!(src, dst);
        let mut dst = dst.iter_mut();

        let mut last_offset = 0;

        for offset in offsets {
            // Safe because we checked dst.len() == num_elements, and num_elements
            // is also the size of offsets
            let dst_item: &mut Option<Vec<I>> = unsafe { dst.next().unsafe_unwrap() };
            match offset {
                None => *dst_item = None,
                Some(range) => {
                    *dst_item = Some(build_list_item!(range, last_offset, elements));
                }
            }
        }
        if elements.next().is_some() {
            panic!("List too long");
        }

        Ok(())
    }
}

/// Deserialization of ORC lists without nullable values
impl<I> OrcDeserialize for Vec<I>
where
    I: OrcDeserialize,
{
    fn read_from_vector_batch<'a, 'b, T>(
        src: &BorrowedColumnVectorBatch,
        mut dst: &'b mut T,
    ) -> Result<(), DeserializationError>
    where
        &'b mut T: DeserializationTarget<'a, Item = Self> + 'b,
    {
        if src.not_null().is_some() {
            // If it is `Some`, there is at least one null so we are going to
            // crash eventually. Exit early to avoid checking every single value
            // later.
            return Err(DeserializationError::UnexpectedNull(format!(
                "{} column contains nulls",
                stringify!($ty)
            )));
        }

        let (offsets, mut elements) = init_list_read!(src, dst);
        let mut dst = dst.iter_mut();

        let mut last_offset = 0;

        for offset in offsets {
            // This is safe because we checked above this column contains no
            // nulls (`offsets.not_null().is_some()`), so `offset` can't be None.
            let range = unsafe { offset.unsafe_unwrap() };

            // Safe because we checked dst.len() == num_elements, and num_elements
            // is also the size of offsets
            let dst_item: &mut Vec<I> = unsafe { dst.next().unsafe_unwrap() };

            *dst_item = build_list_item!(range, last_offset, elements);
        }
        if elements.next().is_some() {
            panic!("List too long");
        }

        Ok(())
    }
}

/// The trait of things that can have ORC data written to them.
///
/// It must be (mutably) iterable, exact-size, and iterable multiple times (one for
/// each column it contains).
pub trait DeserializationTarget<'a> {
    type Item: 'a;
    type IterMut<'b>: Iterator<Item = &'b mut Self::Item>
    where
        Self: 'b,
        'a: 'b;

    fn len(&self) -> usize;
    fn iter_mut(&mut self) -> Self::IterMut<'_>;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn map<B, F>(&mut self, f: F) -> MultiMap<Self, F>
    where
        Self: Sized,
        F: FnMut(&mut Self::Item) -> &mut B,
    {
        MultiMap { iter: self, f }
    }
}

impl<'a, V: Sized + 'a> DeserializationTarget<'a> for &mut Vec<V> {
    type Item = V;
    type IterMut<'b> = IterMut<'b, V> where V: 'b, 'a: 'b, Self: 'b;

    fn len(&self) -> usize {
        (self as &Vec<_>).len()
    }

    fn iter_mut(&mut self) -> IterMut<'_, V> {
        <[_]>::iter_mut(self)
    }
}

/// A map that can be iterated multiple times
pub struct MultiMap<'c, T: Sized, F> {
    iter: &'c mut T,
    f: F,
}

impl<'a, 'c, V: Sized + 'a, V2: Sized + 'a, T, F> DeserializationTarget<'a>
    for &mut MultiMap<'c, T, F>
where
    F: Copy + for<'b> FnMut(&'b mut V) -> &'b mut V2,
    T: DeserializationTarget<'a, Item = V>,
{
    type Item = V2;
    type IterMut<'b> = Map<T::IterMut<'b>, F> where T: 'b, 'a: 'b, F: 'b, Self: 'b;

    fn len(&self) -> usize {
        self.iter.len()
    }

    fn iter_mut(&mut self) -> Map<T::IterMut<'_>, F> {
        self.iter.iter_mut().map(self.f)
    }
}

/// Given a [`StructVectorBatch`], returns a vector of structures initialized with
/// [`Default`] for ever not-null value in the [`StructVectorBatch`], and `None` for
/// null values.
pub fn default_option_vec<T: Default>(vector_batch: &StructVectorBatch) -> Vec<Option<T>> {
    match vector_batch.not_null() {
        None => (0..vector_batch.num_elements())
            .map(|_| Some(Default::default()))
            .collect(),
        Some(not_null) => not_null
            .iter()
            .map(|&b| {
                if b == 0 {
                    None
                } else {
                    Some(Default::default())
                }
            })
            .collect(),
    }
}

/// Internal trait to allow implementing OrcDeserialize on `Option<T>` where `T` is
/// a structure defined in other crates
pub trait OrcDeserializeOption: Sized + CheckableKind {
    /// Reads from a [`BorrowedColumnVectorBatch`] to a structure that behaves like
    /// a rewindable iterator of `&mut Option<Self>`.
    fn read_options_from_vector_batch<'a, 'b, T>(
        src: &BorrowedColumnVectorBatch,
        dst: &'b mut T,
    ) -> Result<(), DeserializationError>
    where
        Self: 'a,
        &'b mut T: DeserializationTarget<'a, Item = Option<Self>> + 'b;
}

impl<I: OrcDeserializeOption> OrcDeserialize for Option<I> {
    fn read_from_vector_batch<'a, 'b, T>(
        src: &BorrowedColumnVectorBatch,
        dst: &'b mut T,
    ) -> Result<(), DeserializationError>
    where
        &'b mut T: DeserializationTarget<'a, Item = Self> + 'b,
        I: 'a,
    {
        I::read_options_from_vector_batch(src, dst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kind::Kind;
    use vector::BorrowedColumnVectorBatch;

    #[test]
    fn test_map_struct() {
        // TODO: for now this test only makes sure the code compiles, but it should
        // actually run it eventually.
        #[derive(Default)]
        struct Test {
            field1: Option<i64>,
        }

        impl CheckableKind for Test {
            fn check_kind(kind: &Kind) -> Result<(), String> {
                check_kind_equals(
                    kind,
                    &Kind::Struct(vec![("field1".to_owned(), Kind::Long)]),
                    "Vec<u8>",
                )
            }
        }

        impl OrcDeserialize for Option<Test> {
            fn read_from_vector_batch<'a, 'b, T>(
                src: &BorrowedColumnVectorBatch,
                mut dst: &'b mut T,
            ) -> Result<(), DeserializationError>
            where
                &'b mut T: DeserializationTarget<'a, Item = Self>,
            {
                let src = src
                    .try_into_structs()
                    .map_err(DeserializationError::MismatchedColumnKind)?;
                let columns = src.fields();
                let column: BorrowedColumnVectorBatch = columns.into_iter().next().unwrap();
                OrcDeserialize::read_from_vector_batch::<MultiMap<&mut T, _>>(
                    &column,
                    &mut dst.map(|struct_| &mut struct_.as_mut().unwrap().field1),
                )?;

                Ok(())
            }
        }
    }

    #[test]
    fn test_check_kind() {
        assert_eq!(i64::check_kind(&Kind::Long), Ok(()));
        assert_eq!(String::check_kind(&Kind::String), Ok(()));
        assert_eq!(Vec::<u8>::check_kind(&Kind::Binary), Ok(()));
    }

    #[test]
    fn test_check_kind_fail() {
        assert_eq!(
            i64::check_kind(&Kind::String),
            Err("i64 must be decoded from ORC Long, not ORC String".to_string())
        );
        assert_eq!(
            i64::check_kind(&Kind::Int),
            Err("i64 must be decoded from ORC Long, not ORC Int".to_string())
        );
        assert_eq!(
            String::check_kind(&Kind::Int),
            Err("String must be decoded from ORC String, not ORC Int".to_string())
        );
        assert_eq!(
            String::check_kind(&Kind::Binary),
            Err("String must be decoded from ORC String, not ORC Binary".to_string())
        );
        assert_eq!(
            Vec::<u8>::check_kind(&Kind::Int),
            Err("Vec<u8> must be decoded from ORC Binary, not ORC Int".to_string())
        );
        assert_eq!(
            Vec::<u8>::check_kind(&Kind::String),
            Err("Vec<u8> must be decoded from ORC Binary, not ORC String".to_string())
        );
    }
}
