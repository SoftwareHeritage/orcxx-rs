// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

//! Contains for columns for values of uniform types

use std::convert::TryInto;
use std::fmt;
use std::marker::PhantomData;
use std::os::raw::c_char;
use std::ptr;

use cxx::UniquePtr;
use rust_decimal::Decimal;

use memorypool;
use utils::{OrcError, OrcResult};

// TODO: remove $function_name when https://github.com/rust-lang/rust/issues/29599
// is stabilized
macro_rules! impl_debug {
    ($struct_name:ident, $function_name:path) => {
        impl fmt::Debug for $struct_name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    concat!(stringify!($struct_name), " {{ {} }}"),
                    $function_name(&self.0)
                )
            }
        }
    };
    ($struct_name:ident<$lifetime:lifetime>, $function_name:path) => {
        impl<$lifetime> fmt::Debug for $struct_name<$lifetime> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    concat!(stringify!($struct_name), " {{ {} }}"),
                    $function_name(&self.0)
                )
            }
        }
    };
}

#[cxx::bridge]
pub(crate) mod ffi {
    // Reimport types from other modules
    #[namespace = "orcxx_rs"]
    unsafe extern "C++" {
        type Int64DataBuffer = crate::memorypool::ffi::Int64DataBuffer;
        type Int128DataBuffer = crate::memorypool::ffi::Int128DataBuffer;
        type DoubleDataBuffer = crate::memorypool::ffi::DoubleDataBuffer;
        type StringDataBuffer = crate::memorypool::ffi::StringDataBuffer;
        type CharDataBuffer = crate::memorypool::ffi::CharDataBuffer;
    }

    #[namespace = "orc"]
    unsafe extern "C++" {
        include!("cpp-utils.hh");
        include!("orc/Vector.hh");

        type ColumnVectorBatch;
        type LongVectorBatch;
        type DoubleVectorBatch;
        type StringVectorBatch;
        type TimestampVectorBatch;
        type Decimal64VectorBatch;
        type Decimal128VectorBatch;
        type StructVectorBatch;
        type ListVectorBatch;
        type MapVectorBatch;
    }

    impl UniquePtr<ColumnVectorBatch> {}

    #[namespace = "orcxx_rs"]
    unsafe extern "C++" {
        type ColumnVectorBatchPtr;

        #[namespace = "orcxx_rs::utils"]
        #[rust_name = "ColumnVectorBatchPtr_make_ptr"]
        fn into(batch_ptr: &ColumnVectorBatchPtr) -> *const ColumnVectorBatch;

    }

    #[namespace = "orcxx_rs::accessors"]
    unsafe extern "C++" {
        fn get_numElements(vectorBatch: &ColumnVectorBatch) -> u64;
        fn get_hasNulls(vectorBatch: &ColumnVectorBatch) -> bool;
        fn get_notNull(vectorBatch: &ColumnVectorBatch) -> &CharDataBuffer;

        #[rust_name = "LongVectorBatch_get_data"]
        fn get_data(vectorBatch: &LongVectorBatch) -> &Int64DataBuffer;

        #[rust_name = "DoubleVectorBatch_get_data"]
        fn get_data(vectorBatch: &DoubleVectorBatch) -> &DoubleDataBuffer;

        #[rust_name = "StringVectorBatch_get_data"]
        fn get_data(vectorBatch: &StringVectorBatch) -> &StringDataBuffer;
        #[rust_name = "StringVectorBatch_get_length"]
        fn get_length(vectorBatch: &StringVectorBatch) -> &Int64DataBuffer;

        #[rust_name = "TimestampVectorBatch_get_data"]
        fn get_data(vectorBatch: &TimestampVectorBatch) -> &Int64DataBuffer;
        #[rust_name = "TimestampVectorBatch_get_nanoseconds"]
        fn get_nanoseconds(vectorBatch: &TimestampVectorBatch) -> &Int64DataBuffer;

        #[rust_name = "Decimal64VectorBatch_get_values"]
        fn get_values(vectorBatch: &Decimal64VectorBatch) -> &Int64DataBuffer;
        #[rust_name = "Decimal64VectorBatch_get_precision"]
        fn get_precision(vectorBatch: &Decimal64VectorBatch) -> i32;
        #[rust_name = "Decimal64VectorBatch_get_scale"]
        fn get_scale(vectorBatch: &Decimal64VectorBatch) -> i32;

        #[rust_name = "Decimal128VectorBatch_get_values"]
        fn get_values(vectorBatch: &Decimal128VectorBatch) -> &Int128DataBuffer;
        #[rust_name = "Decimal128VectorBatch_get_precision"]
        fn get_precision(vectorBatch: &Decimal128VectorBatch) -> i32;
        #[rust_name = "Decimal128VectorBatch_get_scale"]
        fn get_scale(vectorBatch: &Decimal128VectorBatch) -> i32;

        #[rust_name = "StructVectorBatch_get_fields"]
        fn get_fields(vectorBatch: &StructVectorBatch) -> &CxxVector<ColumnVectorBatchPtr>;

        #[rust_name = "ListVectorBatch_get_elements"]
        fn get_elements(vectorBatch: &ListVectorBatch) -> &UniquePtr<ColumnVectorBatch>;
        #[rust_name = "ListVectorBatch_get_offsets"]
        fn get_offsets(vectorBatch: &ListVectorBatch) -> &Int64DataBuffer;

        #[rust_name = "MapVectorBatch_get_keys"]
        fn get_keys(vectorBatch: &MapVectorBatch) -> &UniquePtr<ColumnVectorBatch>;
        #[rust_name = "MapVectorBatch_get_elements"]
        fn get_elements(vectorBatch: &MapVectorBatch) -> &UniquePtr<ColumnVectorBatch>;
        #[rust_name = "MapVectorBatch_get_offsets"]
        fn get_offsets(vectorBatch: &MapVectorBatch) -> &Int64DataBuffer;
    }

    #[namespace = "orcxx_rs::utils"]
    unsafe extern "C++" {
        #[rust_name = "try_into_LongVectorBatch"]
        fn try_into(vectorBatch: &ColumnVectorBatch) -> Result<&LongVectorBatch>;
        #[rust_name = "try_into_DoubleVectorBatch"]
        fn try_into(vectorBatch: &ColumnVectorBatch) -> Result<&DoubleVectorBatch>;
        #[rust_name = "try_into_StringVectorBatch"]
        fn try_into(vectorBatch: &ColumnVectorBatch) -> Result<&StringVectorBatch>;
        #[rust_name = "try_into_TimestampVectorBatch"]
        fn try_into(vectorBatch: &ColumnVectorBatch) -> Result<&TimestampVectorBatch>;
        #[rust_name = "try_into_Decimal64VectorBatch"]
        fn try_into(vectorBatch: &ColumnVectorBatch) -> Result<&Decimal64VectorBatch>;
        #[rust_name = "try_into_Decimal128VectorBatch"]
        fn try_into(vectorBatch: &ColumnVectorBatch) -> Result<&Decimal128VectorBatch>;
        #[rust_name = "try_into_StructVectorBatch"]
        fn try_into(vectorBatch: &ColumnVectorBatch) -> Result<&StructVectorBatch>;
        #[rust_name = "try_into_ListVectorBatch"]
        fn try_into(vectorBatch: &ColumnVectorBatch) -> Result<&ListVectorBatch>;
        #[rust_name = "try_into_MapVectorBatch"]
        fn try_into(vectorBatch: &ColumnVectorBatch) -> Result<&MapVectorBatch>;

        #[rust_name = "LongVectorBatch_into_ColumnVectorBatch"]
        fn try_into(vectorBatch: &LongVectorBatch) -> &ColumnVectorBatch;
        #[rust_name = "DoubleVectorBatch_into_ColumnVectorBatch"]
        fn try_into(vectorBatch: &DoubleVectorBatch) -> &ColumnVectorBatch;
        #[rust_name = "StringVectorBatch_into_ColumnVectorBatch"]
        fn try_into(vectorBatch: &StringVectorBatch) -> &ColumnVectorBatch;
        #[rust_name = "TimestampVectorBatch_into_ColumnVectorBatch"]
        fn try_into(vectorBatch: &TimestampVectorBatch) -> &ColumnVectorBatch;
        #[rust_name = "Decimal64VectorBatch_into_ColumnVectorBatch"]
        fn try_into(vectorBatch: &Decimal64VectorBatch) -> &ColumnVectorBatch;
        #[rust_name = "Decimal128VectorBatch_into_ColumnVectorBatch"]
        fn try_into(vectorBatch: &Decimal128VectorBatch) -> &ColumnVectorBatch;
        #[rust_name = "ListVectorBatch_into_ColumnVectorBatch"]
        fn try_into(vectorBatch: &ListVectorBatch) -> &ColumnVectorBatch;
        #[rust_name = "MapVectorBatch_into_ColumnVectorBatch"]
        fn try_into(vectorBatch: &MapVectorBatch) -> &ColumnVectorBatch;

        #[rust_name = "ColumnVectorBatch_toString"]
        fn toString(type_: &ColumnVectorBatch) -> UniquePtr<CxxString>;
        #[rust_name = "LongVectorBatch_toString"]
        fn toString(type_: &LongVectorBatch) -> UniquePtr<CxxString>;
        #[rust_name = "DoubleVectorBatch_toString"]
        fn toString(type_: &DoubleVectorBatch) -> UniquePtr<CxxString>;
        #[rust_name = "StringVectorBatch_toString"]
        fn toString(type_: &StringVectorBatch) -> UniquePtr<CxxString>;
        #[rust_name = "TimestampVectorBatch_toString"]
        fn toString(type_: &TimestampVectorBatch) -> UniquePtr<CxxString>;
        #[rust_name = "Decimal64VectorBatch_toString"]
        fn toString(type_: &Decimal64VectorBatch) -> UniquePtr<CxxString>;
        #[rust_name = "Decimal128VectorBatch_toString"]
        fn toString(type_: &Decimal128VectorBatch) -> UniquePtr<CxxString>;
        #[rust_name = "StructVectorBatch_toString"]
        fn toString(type_: &StructVectorBatch) -> UniquePtr<CxxString>;
        #[rust_name = "ListVectorBatch_toString"]
        fn toString(type_: &ListVectorBatch) -> UniquePtr<CxxString>;
        #[rust_name = "MapVectorBatch_toString"]
        fn toString(type_: &MapVectorBatch) -> UniquePtr<CxxString>;
    }
}

/// Common methods of [`OwnedColumnVectorBatch`] and [`BorrowedColumnVectorBatch`]
pub trait ColumnVectorBatch {
    fn inner(&self) -> &ffi::ColumnVectorBatch;

    fn num_elements(&self) -> u64 {
        ffi::get_numElements(self.inner())
    }

    /// If the vector contains any null value, then returns an array of booleans
    /// indicating whether each row is null (and should be skipped when reading
    /// it) or not.
    ///
    /// See [`BorrowedColumnVectorBatch::not_null`] to get a slice.
    fn not_null_ptr(&self) -> Option<ptr::NonNull<i8>> {
        if ffi::get_hasNulls(self.inner()) {
            let not_null = ffi::get_notNull(self.inner()).data();
            assert_ne!(not_null, ptr::null());

            // This is safe because we just checked it is not null
            Some(unsafe { ptr::NonNull::new_unchecked(not_null as *mut i8) })
        } else {
            None
        }
    }
}

/// A column (or set of column) of a stripe, with values of unknown type.
pub struct OwnedColumnVectorBatch(pub(crate) UniquePtr<ffi::ColumnVectorBatch>);

impl_debug!(OwnedColumnVectorBatch, ffi::ColumnVectorBatch_toString);

impl ColumnVectorBatch for OwnedColumnVectorBatch {
    fn inner(&self) -> &ffi::ColumnVectorBatch {
        &self.0
    }
}

impl OwnedColumnVectorBatch {
    pub fn borrow(&mut self) -> BorrowedColumnVectorBatch {
        BorrowedColumnVectorBatch(&self.0)
    }
}

/// A column (or set of column) of a stripe, with values of unknown type.
pub struct BorrowedColumnVectorBatch<'a>(&'a ffi::ColumnVectorBatch);

impl_debug!(
    BorrowedColumnVectorBatch<'a>,
    ffi::ColumnVectorBatch_toString
);

impl<'a> ColumnVectorBatch for BorrowedColumnVectorBatch<'a> {
    fn inner(&self) -> &ffi::ColumnVectorBatch {
        self.0
    }
}

impl<'a> BorrowedColumnVectorBatch<'a> {
    /// Same as [ColumnVectorBatch::not_null_ptr] but returns a slice.
    pub fn not_null(&self) -> Option<&'a [i8]> {
        if ffi::get_hasNulls(self.inner()) {
            let num_elements = self
                .num_elements()
                .try_into()
                .expect("could not convert u64 to usize");
            let not_null = ffi::get_notNull(self.inner()).data();

            // This is safe because we just checked it is not null
            Some(unsafe { std::slice::from_raw_parts(not_null, num_elements) })
        } else {
            None
        }
    }
    pub fn try_into_longs(&self) -> OrcResult<LongVectorBatch<'a>> {
        ffi::try_into_LongVectorBatch(self.0)
            .map_err(OrcError)
            .map(LongVectorBatch)
    }

    pub fn try_into_doubles(&self) -> OrcResult<DoubleVectorBatch<'a>> {
        ffi::try_into_DoubleVectorBatch(self.0)
            .map_err(OrcError)
            .map(DoubleVectorBatch)
    }

    pub fn try_into_strings(&self) -> OrcResult<StringVectorBatch<'a>> {
        ffi::try_into_StringVectorBatch(self.0)
            .map_err(OrcError)
            .map(StringVectorBatch)
    }

    pub fn try_into_timestamps(&self) -> OrcResult<TimestampVectorBatch<'a>> {
        ffi::try_into_TimestampVectorBatch(self.0)
            .map_err(OrcError)
            .map(TimestampVectorBatch)
    }

    pub fn try_into_decimals64(&self) -> OrcResult<Decimal64VectorBatch<'a>> {
        ffi::try_into_Decimal64VectorBatch(self.0)
            .map_err(OrcError)
            .map(Decimal64VectorBatch)
    }

    pub fn try_into_decimals128(&self) -> OrcResult<Decimal128VectorBatch<'a>> {
        ffi::try_into_Decimal128VectorBatch(self.0)
            .map_err(OrcError)
            .map(Decimal128VectorBatch)
    }

    pub fn try_into_structs(&self) -> OrcResult<StructVectorBatch<'a>> {
        ffi::try_into_StructVectorBatch(self.0)
            .map_err(OrcError)
            .map(StructVectorBatch)
    }

    pub fn try_into_lists(&self) -> OrcResult<ListVectorBatch<'a>> {
        ffi::try_into_ListVectorBatch(self.0)
            .map_err(OrcError)
            .map(ListVectorBatch)
    }

    pub fn try_into_maps(&self) -> OrcResult<MapVectorBatch<'a>> {
        ffi::try_into_MapVectorBatch(self.0)
            .map_err(OrcError)
            .map(MapVectorBatch)
    }
}

/// A specialized [`ColumnVectorBatch`] whose values are known to be structures.
///
/// It is constructed through [`BorrowedColumnVectorBatch::try_into_structs`]
pub struct StructVectorBatch<'a>(&'a ffi::StructVectorBatch);

impl_debug!(StructVectorBatch<'a>, ffi::StructVectorBatch_toString);

impl<'a> StructVectorBatch<'a> {
    pub fn fields(&self) -> Vec<BorrowedColumnVectorBatch<'a>> {
        ffi::StructVectorBatch_get_fields(&self.0)
            .iter()
            .map(|batch_ptr| {
                BorrowedColumnVectorBatch(unsafe {
                    // This is safe because the dereferenced ColumnVectorBatch will
                    // live as long as StructVectorBatch is not overwritten or freeed,
                    // which it cannot be as the dereferenced ColumnVectorBatch has
                    // a lifetime shorter than this StructVectorBatch
                    &*ffi::ColumnVectorBatchPtr_make_ptr(batch_ptr)
                })
            })
            .collect()
    }
}

/// A specialized [`ColumnVectorBatch`] whose values are known to be integer-like.
///
/// It is constructed through [`BorrowedColumnVectorBatch::try_into_longs`]
pub struct LongVectorBatch<'a>(&'a ffi::LongVectorBatch);

impl_debug!(LongVectorBatch<'a>, ffi::LongVectorBatch_toString);

impl<'a> LongVectorBatch<'a> {
    pub fn iter(&self) -> LongVectorBatchIterator {
        let data = ffi::LongVectorBatch_get_data(self.0);
        let vector_batch =
            BorrowedColumnVectorBatch(ffi::LongVectorBatch_into_ColumnVectorBatch(&self.0));
        let num_elements = vector_batch.num_elements();
        let not_null = vector_batch.not_null_ptr();

        unsafe { LongVectorBatchIterator::new(data, not_null, num_elements) }
    }
}

/// Iterator on [`LongVectorBatch`]
#[derive(Debug, Clone)]
pub struct LongVectorBatchIterator<'a> {
    batch: PhantomData<&'a LongVectorBatch<'a>>,
    data_index: isize,
    not_null_index: isize,
    data: *const i64,
    not_null: Option<ptr::NonNull<i8>>,
    num_elements: isize,
}

impl<'a> LongVectorBatchIterator<'a> {
    unsafe fn new(
        data_buffer: &memorypool::ffi::Int64DataBuffer,
        not_null: Option<ptr::NonNull<i8>>,
        num_elements: u64,
    ) -> LongVectorBatchIterator<'a> {
        // TODO: do this once https://github.com/apache/orc/commit/294a5e28f7f0420eb1fdc76dffc33608692c1b20
        // is released:
        // assert_eq!(std::mem::size_of(u64)*num_elements, data_buffer.size())
        LongVectorBatchIterator {
            batch: PhantomData,
            data_index: 0,
            not_null_index: 0,
            data: data_buffer.data(),
            not_null,
            num_elements: num_elements
                .try_into()
                .expect("could not convert u64 to isize"),
        }
    }
}

impl<'a> Iterator for LongVectorBatchIterator<'a> {
    type Item = Option<i64>;

    fn next(&mut self) -> Option<Option<i64>> {
        if self.not_null_index >= self.num_elements {
            return None;
        }

        if let Some(not_null) = self.not_null {
            let not_null = not_null.as_ptr();
            // This is should be safe because we just checked not_null_index is lower
            // than self.num_elements, which is the length of 'not_null'
            if unsafe { *not_null.offset(self.not_null_index) } == 0 {
                self.not_null_index += 1;
                return Some(None);
            }
        }

        self.not_null_index += 1;

        // This should be safe because 'num_elements' should be exactly
        // the number of element in the array plus the number of nulls that we skipped,
        // and we checked 'index' is lower than 'num_elements'.
        let datum = unsafe { *self.data.offset(self.data_index) };

        self.data_index += 1;

        Some(Some(datum))
    }
}

/// A specialized [`ColumnVectorBatch`] whose values are known to be floating-point-like
///
/// It is constructed through [`BorrowedColumnVectorBatch::try_into_doubles`]
pub struct DoubleVectorBatch<'a>(&'a ffi::DoubleVectorBatch);

impl_debug!(DoubleVectorBatch<'a>, ffi::DoubleVectorBatch_toString);

impl<'a> DoubleVectorBatch<'a> {
    pub fn iter(&self) -> DoubleVectorBatchIterator {
        let data = ffi::DoubleVectorBatch_get_data(self.0).data();
        let vector_batch =
            BorrowedColumnVectorBatch(ffi::DoubleVectorBatch_into_ColumnVectorBatch(&self.0));
        let num_elements = vector_batch.num_elements();
        let not_null = vector_batch.not_null_ptr();

        DoubleVectorBatchIterator {
            batch: PhantomData,
            data_index: 0,
            not_null_index: 0,
            data,
            not_null,
            num_elements: num_elements
                .try_into()
                .expect("could not convert u64 to isize"),
        }
    }
}

/// Iterator on [`DoubleVectorBatch`]
#[derive(Debug, Clone)]
pub struct DoubleVectorBatchIterator<'a> {
    batch: PhantomData<&'a DoubleVectorBatch<'a>>,
    data_index: isize,
    not_null_index: isize,
    data: *const f64,
    not_null: Option<ptr::NonNull<i8>>,
    num_elements: isize,
}

impl<'a> Iterator for DoubleVectorBatchIterator<'a> {
    type Item = Option<f64>;

    fn next(&mut self) -> Option<Option<f64>> {
        if self.not_null_index >= self.num_elements {
            return None;
        }

        if let Some(not_null) = self.not_null {
            let not_null = not_null.as_ptr();
            // This is should be safe because we just checked not_null_index is lower
            // than self.num_elements, which is the length of 'not_null'
            if unsafe { *not_null.offset(self.not_null_index) } == 0 {
                self.not_null_index += 1;
                return Some(None);
            }
        }

        self.not_null_index += 1;

        // This should be safe because 'num_elements' should be exactly
        // the number of element in the array plus the number of nulls that we skipped,
        // and we checked 'index' is lower than 'num_elements'.
        let datum = unsafe { *self.data.offset(self.data_index) };

        self.data_index += 1;

        Some(Some(datum))
    }
}

/// A specialized [`ColumnVectorBatch`] whose values are known to be string-like.
///
/// It is constructed through [`BorrowedColumnVectorBatch::try_into_strings`]
pub struct StringVectorBatch<'a>(&'a ffi::StringVectorBatch);

impl_debug!(StringVectorBatch<'a>, ffi::StringVectorBatch_toString);

impl<'a> StringVectorBatch<'a> {
    pub fn iter(&self) -> StringVectorBatchIterator {
        let data = ffi::StringVectorBatch_get_data(self.0).data();
        let lengths = ffi::StringVectorBatch_get_length(self.0).data();
        let vector_batch =
            BorrowedColumnVectorBatch(ffi::StringVectorBatch_into_ColumnVectorBatch(&self.0));
        let num_elements = vector_batch.num_elements();
        let not_null = vector_batch.not_null_ptr();

        StringVectorBatchIterator {
            batch: PhantomData,
            index: 0,
            data,
            not_null,
            lengths,
            num_elements: num_elements
                .try_into()
                .expect("could not convert u64 to isize"),
        }
    }
}

/// Iterator on [`StringVectorBatch`]
#[derive(Debug, Clone)]
pub struct StringVectorBatchIterator<'a> {
    batch: PhantomData<&'a StringVectorBatch<'a>>,
    index: isize,
    data: *const *mut c_char, // Pointers to start of strings
    lengths: *const i64,      // Length of each string
    not_null: Option<ptr::NonNull<i8>>,
    num_elements: isize,
}

impl<'a> Iterator for StringVectorBatchIterator<'a> {
    type Item = Option<&'a [u8]>;

    fn next(&mut self) -> Option<Option<&'a [u8]>> {
        if self.index >= self.num_elements {
            return None;
        }

        if let Some(not_null) = self.not_null {
            let not_null = not_null.as_ptr();
            // This is should be safe because we just checked not_null_index is lower
            // than self.num_elements, which is the length of 'not_null'
            if unsafe { *not_null.offset(self.index) } == 0 {
                self.index += 1;
                return Some(None);
            }
        }

        // These two should be safe because 'num_elements' should be exactly
        // the number of element in each array, and we checked 'index' is lower than
        // 'num_elements'.
        let datum = unsafe { *self.data.offset(self.index) };
        let length = unsafe { *self.lengths.offset(self.index) };

        self.index += 1;

        let length = length.try_into().expect("could not convert u64 to usize");

        // Should be safe because the length indicates the number of bytes in
        // the string.
        let datum = datum as *const u8;
        Some(Some(unsafe { std::slice::from_raw_parts(datum, length) }))
    }
}

/// A specialized [`ColumnVectorBatch`] whose values are known to be timestamps,
/// represented by seconds and nanoseconds since 1970-01-01 GMT.
///
/// It is constructed through [`BorrowedColumnVectorBatch::try_into_timestamps`]
pub struct TimestampVectorBatch<'a>(&'a ffi::TimestampVectorBatch);

impl_debug!(TimestampVectorBatch<'a>, ffi::TimestampVectorBatch_toString);

impl<'a> TimestampVectorBatch<'a> {
    pub fn iter(&self) -> TimestampVectorBatchIterator {
        let data = ffi::TimestampVectorBatch_get_data(self.0).data();
        let nanoseconds = ffi::TimestampVectorBatch_get_nanoseconds(self.0).data();
        let vector_batch =
            BorrowedColumnVectorBatch(ffi::TimestampVectorBatch_into_ColumnVectorBatch(&self.0));
        let num_elements = vector_batch.num_elements();
        let not_null = vector_batch.not_null_ptr();

        TimestampVectorBatchIterator {
            batch: PhantomData,
            index: 0,
            data,
            not_null,
            nanoseconds,
            num_elements: num_elements
                .try_into()
                .expect("could not convert u64 to isize"),
        }
    }
}

/// Iterator on [`TimestampVectorBatch`]
#[derive(Debug, Clone)]
pub struct TimestampVectorBatchIterator<'a> {
    batch: PhantomData<&'a TimestampVectorBatch<'a>>,
    index: isize,
    data: *const i64, // Seconds since 1970-01-01
    nanoseconds: *const i64,
    not_null: Option<ptr::NonNull<i8>>,
    num_elements: isize,
}

impl<'a> Iterator for TimestampVectorBatchIterator<'a> {
    type Item = Option<(i64, i64)>;

    fn next(&mut self) -> Option<Option<(i64, i64)>> {
        if self.index >= self.num_elements {
            return None;
        }

        if let Some(not_null) = self.not_null {
            let not_null = not_null.as_ptr();
            // This is should be safe because we just checked not_null_index is lower
            // than self.num_elements, which is the length of 'not_null'
            if unsafe { *not_null.offset(self.index) } == 0 {
                self.index += 1;
                return Some(None);
            }
        }

        // These two should be safe because 'num_elements' should be exactly
        // the number of element in each array, and we checked 'index' is lower than
        // 'num_elements'.
        let datum = unsafe { *self.data.offset(self.index) };
        let nanoseconds = unsafe { *self.nanoseconds.offset(self.index) };

        self.index += 1;

        Some(Some((datum, nanoseconds)))
    }
}

/// Common methods of [`Decimal64VectorBatch`] and [`Decimal128VectorBatch`]
pub trait DecimalVectorBatch<'a> {
    type IteratorType: Iterator<Item = Option<Decimal>>;

    /// total number of digits
    fn precision(&self) -> i32;
    /// the number of places after the decimal
    fn scale(&self) -> i32;
    fn iter(&self) -> Self::IteratorType;
}

/// A specialized [`ColumnVectorBatch`] whose values are known to be 64-bits decimal numbers
///
/// It is constructed through [`BorrowedColumnVectorBatch::try_into_doubles`]
pub struct Decimal64VectorBatch<'a>(&'a ffi::Decimal64VectorBatch);

impl_debug!(Decimal64VectorBatch<'a>, ffi::Decimal64VectorBatch_toString);

impl<'a> DecimalVectorBatch<'a> for Decimal64VectorBatch<'a> {
    type IteratorType = Decimal64VectorBatchIterator<'a>;

    fn precision(&self) -> i32 {
        ffi::Decimal64VectorBatch_get_precision(self.0)
    }

    fn scale(&self) -> i32 {
        ffi::Decimal64VectorBatch_get_scale(self.0)
    }

    fn iter(&self) -> Decimal64VectorBatchIterator<'a> {
        let data = ffi::Decimal64VectorBatch_get_values(self.0).data();
        let vector_batch =
            BorrowedColumnVectorBatch(ffi::Decimal64VectorBatch_into_ColumnVectorBatch(&self.0));
        let num_elements = vector_batch.num_elements();
        let not_null = vector_batch.not_null_ptr();

        Decimal64VectorBatchIterator {
            batch: PhantomData,
            data_index: 0,
            not_null_index: 0,
            data,
            not_null,
            num_elements: num_elements
                .try_into()
                .expect("could not convert u64 to isize"),
            scale: self
                .scale()
                .try_into()
                .expect("Could not convert scale from i32 to u43"),
        }
    }
}

/// Iterator on [`Decimal64VectorBatch`]
#[derive(Debug, Clone)]
pub struct Decimal64VectorBatchIterator<'a> {
    batch: PhantomData<&'a Decimal64VectorBatch<'a>>,
    data_index: isize,
    not_null_index: isize,
    data: *const i64,
    not_null: Option<ptr::NonNull<i8>>,
    num_elements: isize,
    scale: u32,
}

impl<'a> Iterator for Decimal64VectorBatchIterator<'a> {
    type Item = Option<Decimal>;

    fn next(&mut self) -> Option<Option<Decimal>> {
        if self.not_null_index >= self.num_elements {
            return None;
        }

        if let Some(not_null) = self.not_null {
            let not_null = not_null.as_ptr();
            // This is should be safe because we just checked not_null_index is lower
            // than self.num_elements, which is the length of 'not_null'
            if unsafe { *not_null.offset(self.not_null_index) } == 0 {
                self.not_null_index += 1;
                return Some(None);
            }
        }

        self.not_null_index += 1;

        // This should be safe because 'num_elements' should be exactly
        // the number of element in the array plus the number of nulls that we skipped,
        // and we checked 'index' is lower than 'num_elements'.
        let datum = unsafe { *self.data.offset(self.data_index) };

        self.data_index += 1;

        Some(Some(Decimal::new(datum, self.scale)))
    }
}

/// A specialized [`ColumnVectorBatch`] whose values are known to be 64-bits decimal numbers
///
/// It is constructed through [`BorrowedColumnVectorBatch::try_into_doubles`]
pub struct Decimal128VectorBatch<'a>(&'a ffi::Decimal128VectorBatch);

impl_debug!(
    Decimal128VectorBatch<'a>,
    ffi::Decimal128VectorBatch_toString
);

impl<'a> DecimalVectorBatch<'a> for Decimal128VectorBatch<'a> {
    type IteratorType = Decimal128VectorBatchIterator<'a>;

    fn precision(&self) -> i32 {
        ffi::Decimal128VectorBatch_get_precision(self.0)
    }

    fn scale(&self) -> i32 {
        ffi::Decimal128VectorBatch_get_scale(self.0)
    }

    fn iter(&self) -> Decimal128VectorBatchIterator<'a> {
        let data = ffi::Decimal128VectorBatch_get_values(self.0).data();
        let vector_batch =
            BorrowedColumnVectorBatch(ffi::Decimal128VectorBatch_into_ColumnVectorBatch(&self.0));
        let num_elements = vector_batch.num_elements();
        let not_null = vector_batch.not_null_ptr();

        Decimal128VectorBatchIterator {
            batch: PhantomData,
            data_index: 0,
            not_null_index: 0,
            data,
            not_null,
            num_elements: num_elements
                .try_into()
                .expect("could not convert u64 to isize"),
            scale: self
                .scale()
                .try_into()
                .expect("Could not convert scale from i32 to u43"),
        }
    }
}

/// Iterator on [`Decimal128VectorBatch`]
#[derive(Debug, Clone)]
pub struct Decimal128VectorBatchIterator<'a> {
    batch: PhantomData<&'a Decimal128VectorBatch<'a>>,
    data_index: isize,
    not_null_index: isize,
    data: *const memorypool::ffi::Int128,
    not_null: Option<ptr::NonNull<i8>>,
    num_elements: isize,
    scale: u32,
}

impl<'a> Iterator for Decimal128VectorBatchIterator<'a> {
    type Item = Option<Decimal>;

    fn next(&mut self) -> Option<Option<Decimal>> {
        if self.not_null_index >= self.num_elements {
            return None;
        }

        if let Some(not_null) = self.not_null {
            let not_null = not_null.as_ptr();
            // This is should be safe because we just checked not_null_index is lower
            // than self.num_elements, which is the length of 'not_null'
            if unsafe { *not_null.offset(self.not_null_index) } == 0 {
                self.not_null_index += 1;
                return Some(None);
            }
        }

        self.not_null_index += 1;

        // This should be safe because 'num_elements' should be exactly
        // the number of element in the array plus the number of nulls that we skipped,
        // and we checked 'index' is lower than 'num_elements'.
        //
        // We need to do a round-trip of conversion through i128 because Int128 is
        // opaque, so it is not sized, so .offset() would just return the initial
        // pointer.
        let datum = unsafe {
            &*((self.data as *const i128).offset(self.data_index) as *const memorypool::ffi::Int128)
        };

        self.data_index += 1;

        let datum = (datum.getHighBits() as i128) << 64 | (datum.getLowBits() as i128);

        Some(Some(Decimal::from_i128_with_scale(datum, self.scale)))
    }
}

/// A specialized [`ColumnVectorBatch`] whose values are lists of other values
///
/// It is constructed through [`BorrowedColumnVectorBatch::try_into_lists`]
pub struct ListVectorBatch<'a>(&'a ffi::ListVectorBatch);

impl_debug!(ListVectorBatch<'a>, ffi::ListVectorBatch_toString);

impl<'a> ListVectorBatch<'a> {
    /// The flat vector of all elements of all lists
    pub fn elements(&self) -> BorrowedColumnVectorBatch<'a> {
        // TODO: notNull
        BorrowedColumnVectorBatch(ffi::ListVectorBatch_get_elements(self.0))
    }

    /// Offset of each ist in the flat vector. None values indicate absent lists
    pub fn iter_offsets(&self) -> LongVectorBatchIterator<'a> {
        let offsets = ffi::ListVectorBatch_get_offsets(self.0);
        let vector_batch =
            BorrowedColumnVectorBatch(ffi::ListVectorBatch_into_ColumnVectorBatch(&self.0));
        let num_elements = vector_batch.num_elements();
        let not_null = vector_batch.not_null_ptr();

        unsafe { LongVectorBatchIterator::new(offsets, not_null, num_elements) }
    }
}

/// A specialized [`ColumnVectorBatch`] whose values are dictionaries with arbitrary types
/// as keys and values
///
/// It is constructed through [`BorrowedColumnVectorBatch::try_into_maps`]
pub struct MapVectorBatch<'a>(&'a ffi::MapVectorBatch);

impl_debug!(MapVectorBatch<'a>, ffi::MapVectorBatch_toString);

impl<'a> MapVectorBatch<'a> {
    /// The flat vector of all keys of all maps
    pub fn keys(&self) -> BorrowedColumnVectorBatch<'a> {
        // TODO: notNull
        BorrowedColumnVectorBatch(ffi::MapVectorBatch_get_keys(self.0))
    }

    /// The flat vector of all values of all maps
    pub fn elements(&self) -> BorrowedColumnVectorBatch<'a> {
        // TODO: notNull
        BorrowedColumnVectorBatch(ffi::MapVectorBatch_get_elements(self.0))
    }

    /// Offset of each ist in the flat vector. None values indicate absent maps
    pub fn iter_offsets(&self) -> LongVectorBatchIterator<'a> {
        let offsets = ffi::MapVectorBatch_get_offsets(self.0);
        let vector_batch =
            BorrowedColumnVectorBatch(ffi::MapVectorBatch_into_ColumnVectorBatch(&self.0));
        let num_elements = vector_batch.num_elements();
        let not_null = vector_batch.not_null_ptr();

        unsafe { LongVectorBatchIterator::new(offsets, not_null, num_elements) }
    }
}
