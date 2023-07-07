// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

//! Helpers for the `orcxx_derive` crate.

use std::convert::TryInto;
use std::iter::Map;
use std::num::TryFromIntError;
use std::slice::IterMut;
use std::str::Utf8Error;

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
    /// [`Vec::from_vector_batch`](OrcDeserializable::options_from_vector_batch) was
    /// called on a non-empty [`Vec`]
    NonEmptyVector,
    /// Failed to decode a [`String`] (use [`Vec<u8>`](`Vec`) instead for columns of
    /// `binary` type).
    Utf8Error(Utf8Error),
}

/// Types which can be read in batch from ORC columns ([`BorrowedColumnVectorBatch`]).
pub trait OrcDeserializable: Sized + Default {
    /*
    fn read_from_vector_batch<'a, T: DeserializationTarget<'a, Inner=Self>>(
        src: &BorrowedColumnVectorBatch,
        dst: T,
    ) -> Result<(), DeserializationError>;
    */

    /// Reads from a [`BorrowedColumnVectorBatch`] to a structure that behaves like
    /// a rewindable iterator of `&mut Option<Self>`.
    fn read_options_from_vector_batch<'a, 'b, T>(
        src: &BorrowedColumnVectorBatch,
        dst: &'b mut T,
    ) -> Result<(), DeserializationError>
    where
        Self: 'a,
        &'b mut T: DeserializationTarget<'a, Item = Option<Self>> + 'b;

    /// Reads from a [`BorrowedColumnVectorBatch`] and returns a `Vec<Option<Self>>`
    ///
    /// This is a wrapper for
    /// [`read_options_from_vector_batch`](OrcDeserializable::read_options_from_vector_batch)
    /// which takes care of allocating a buffer, and returns it.
    fn options_from_vector_batch(
        vector_batch: &BorrowedColumnVectorBatch,
    ) -> Result<Vec<Option<Self>>, DeserializationError> {
        let num_elements = vector_batch.num_elements();
        let num_elements = num_elements
            .try_into()
            .map_err(DeserializationError::UsizeOverflow)?;
        let mut values = Vec::with_capacity(num_elements);
        values.resize_with(num_elements, Default::default);
        Self::read_options_from_vector_batch(vector_batch, &mut values)?;
        Ok(values)
    }
}

impl OrcDeserializable for i64 {
    fn read_options_from_vector_batch<'a, 'b, T>(
        src: &BorrowedColumnVectorBatch,
        mut dst: &'b mut T,
    ) -> Result<(), DeserializationError>
    where
        &'b mut T: DeserializationTarget<'a, Item = Option<Self>> + 'b,
    {
        let src = src
            .try_into_longs()
            .map_err(DeserializationError::MismatchedColumnKind)?;
        for (s, d) in src.iter().zip(dst.iter_mut()) {
            *d = s
        }

        Ok(())
    }
}

impl OrcDeserializable for String {
    fn read_options_from_vector_batch<'a, 'b, T>(
        src: &BorrowedColumnVectorBatch,
        mut dst: &'b mut T,
    ) -> Result<(), DeserializationError>
    where
        &'b mut T: DeserializationTarget<'a, Item = Option<Self>> + 'b,
    {
        let src = src
            .try_into_strings()
            .map_err(DeserializationError::MismatchedColumnKind)?;
        for (s, d) in src.iter().zip(dst.iter_mut()) {
            *d = match s {
                None => None,
                Some(s) => Some(
                    std::str::from_utf8(s)
                        .map_err(DeserializationError::Utf8Error)?
                        .to_string()
                ),
            }
        }

        Ok(())
    }
}

impl OrcDeserializable for Vec<u8> {
    fn read_options_from_vector_batch<'a, 'b, T>(
        src: &BorrowedColumnVectorBatch,
        mut dst: &'b mut T,
    ) -> Result<(), DeserializationError>
    where
        &'b mut T: DeserializationTarget<'a, Item = Option<Self>> + 'b,
    {
        let src = src
            .try_into_strings()
            .map_err(DeserializationError::MismatchedColumnKind)?;
        for (s, d) in src.iter().zip(dst.iter_mut()) {
            *d = s.map(|s| s.to_vec());
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
    fn iter_mut<'b>(&'b mut self) -> Self::IterMut<'b>;

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

    fn iter_mut<'b>(&'b mut self) -> IterMut<'b, V> {
        <[_]>::iter_mut(self)
    }
}

/// A map that can be iterated multiple times
pub struct MultiMap<'c, T: Sized, F> {
    iter: &'c mut T,
    f: F,
}

impl<'a, 'c, V: Sized + 'a, V2: Sized + 'a, T, F> DeserializationTarget<'a> for &mut MultiMap<'c, T, F>
where
    F: Copy + for<'b> FnMut(&'b mut V) -> &'b mut V2,
    T: DeserializationTarget<'a, Item = V>,
{
    type Item = V2;
    type IterMut<'b> = Map<T::IterMut<'b>, F> where T: 'b, 'a: 'b, F: 'b, Self: 'b;

    fn len(&self) -> usize {
        self.iter.len()
    }

    fn iter_mut<'b>(&'b mut self) -> Map<T::IterMut<'b>, F> {
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
            .into_iter()
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

#[cfg(test)]
mod tests {
    use super::*;
    use vector::BorrowedColumnVectorBatch;

    #[test]
    fn test_map_struct() {
        // TODO: for now this test only makes sure the code compiles, but it should
        // actually run it eventually.
        #[derive(Default)]
        struct Test {
            field1: Option<i64>,
        }

        impl OrcDeserializable for Test {
            fn read_options_from_vector_batch<'a, 'b, T>(
                src: &BorrowedColumnVectorBatch,
                mut dst: &'b mut T,
            ) -> Result<(), DeserializationError>
            where
                &'b mut T: DeserializationTarget<'a, Item = Option<Test>>,
            {
                let src = src
                    .try_into_structs()
                    .map_err(DeserializationError::MismatchedColumnKind)?;
                let columns = src.fields();
                let column: BorrowedColumnVectorBatch = columns.into_iter().next().unwrap();
                OrcDeserializable::read_options_from_vector_batch::<MultiMap<&mut T, _>>(
                    &column,
                    &mut dst.map(|struct_| &mut struct_.as_mut().unwrap().field1),
                )?;

                Ok(())
            }
        }
    }
}
