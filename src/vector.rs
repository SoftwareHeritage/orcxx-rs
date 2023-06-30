// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

//! Contains for columns for values of uniform types

use std::convert::TryInto;
use std::fmt;
use std::marker::PhantomData;
use std::os::raw::c_char;

use cxx::UniquePtr;

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
        type DoubleDataBuffer = crate::memorypool::ffi::DoubleDataBuffer;
        type StringDataBuffer = crate::memorypool::ffi::StringDataBuffer;
    }

    #[namespace = "orc"]
    unsafe extern "C++" {
        include!("cpp-utils.hh");
        include!("orc/Vector.hh");

        type ColumnVectorBatch;
        type LongVectorBatch;
        type DoubleVectorBatch;
        type StringVectorBatch;
        type StructVectorBatch;
        type ListVectorBatch;
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

        #[rust_name = "LongVectorBatch_get_data"]
        fn get_data(vectorBatch: &LongVectorBatch) -> &Int64DataBuffer;

        #[rust_name = "DoubleVectorBatch_get_data"]
        fn get_data(vectorBatch: &DoubleVectorBatch) -> &DoubleDataBuffer;

        #[rust_name = "StringVectorBatch_get_data"]
        fn get_data(vectorBatch: &StringVectorBatch) -> &StringDataBuffer;
        #[rust_name = "StringVectorBatch_get_length"]
        fn get_length(vectorBatch: &StringVectorBatch) -> &Int64DataBuffer;

        #[rust_name = "StructVectorBatch_get_fields"]
        fn get_fields(vectorBatch: &StructVectorBatch) -> &CxxVector<ColumnVectorBatchPtr>;

        #[rust_name = "ListVectorBatch_get_elements"]
        fn get_elements(vectorBatch: &ListVectorBatch) -> &UniquePtr<ColumnVectorBatch>;
        #[rust_name = "ListVectorBatch_get_offsets"]
        fn get_offsets(vectorBatch: &ListVectorBatch) -> &Int64DataBuffer;
    }

    #[namespace = "orcxx_rs::utils"]
    unsafe extern "C++" {
        #[rust_name = "try_into_LongVectorBatch"]
        fn try_into(vectorBatch: &ColumnVectorBatch) -> Result<&LongVectorBatch>;
        #[rust_name = "try_into_DoubleVectorBatch"]
        fn try_into(vectorBatch: &ColumnVectorBatch) -> Result<&DoubleVectorBatch>;
        #[rust_name = "try_into_StringVectorBatch"]
        fn try_into(vectorBatch: &ColumnVectorBatch) -> Result<&StringVectorBatch>;
        #[rust_name = "try_into_StructVectorBatch"]
        fn try_into(vectorBatch: &ColumnVectorBatch) -> Result<&StructVectorBatch>;
        #[rust_name = "try_into_ListVectorBatch"]
        fn try_into(vectorBatch: &ColumnVectorBatch) -> Result<&ListVectorBatch>;

        #[rust_name = "LongVectorBatch_into_ColumnVectorBatch"]
        fn try_into(vectorBatch: &LongVectorBatch) -> &ColumnVectorBatch;
        #[rust_name = "DoubleVectorBatch_into_ColumnVectorBatch"]
        fn try_into(vectorBatch: &DoubleVectorBatch) -> &ColumnVectorBatch;
        #[rust_name = "StringVectorBatch_into_ColumnVectorBatch"]
        fn try_into(vectorBatch: &StringVectorBatch) -> &ColumnVectorBatch;
        #[rust_name = "ListVectorBatch_into_ColumnVectorBatch"]
        fn try_into(vectorBatch: &ListVectorBatch) -> &ColumnVectorBatch;

        #[rust_name = "ColumnVectorBatch_toString"]
        fn toString(type_: &ColumnVectorBatch) -> UniquePtr<CxxString>;
        #[rust_name = "LongVectorBatch_toString"]
        fn toString(type_: &LongVectorBatch) -> UniquePtr<CxxString>;
        #[rust_name = "DoubleVectorBatch_toString"]
        fn toString(type_: &DoubleVectorBatch) -> UniquePtr<CxxString>;
        #[rust_name = "StringVectorBatch_toString"]
        fn toString(type_: &StringVectorBatch) -> UniquePtr<CxxString>;
        #[rust_name = "StructVectorBatch_toString"]
        fn toString(type_: &StructVectorBatch) -> UniquePtr<CxxString>;
        #[rust_name = "ListVectorBatch_toString"]
        fn toString(type_: &ListVectorBatch) -> UniquePtr<CxxString>;
    }
}

/// Common methods of [OwnedColumnVectorBatch] and [BorrowedColumnVectorBatch]
pub trait ColumnVectorBatch {
    fn inner(&self) -> &ffi::ColumnVectorBatch;

    fn num_elements(&self) -> u64 {
        ffi::get_numElements(self.inner())
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
    pub fn try_into_longs(self) -> OrcResult<LongVectorBatch<'a>> {
        ffi::try_into_LongVectorBatch(self.0)
            .map_err(OrcError)
            .map(LongVectorBatch)
    }

    pub fn try_into_doubles(self) -> OrcResult<DoubleVectorBatch<'a>> {
        ffi::try_into_DoubleVectorBatch(self.0)
            .map_err(OrcError)
            .map(DoubleVectorBatch)
    }

    pub fn try_into_strings(self) -> OrcResult<StringVectorBatch<'a>> {
        ffi::try_into_StringVectorBatch(self.0)
            .map_err(OrcError)
            .map(StringVectorBatch)
    }

    pub fn try_into_structs(self) -> OrcResult<StructVectorBatch<'a>> {
        ffi::try_into_StructVectorBatch(self.0)
            .map_err(OrcError)
            .map(StructVectorBatch)
    }

    pub fn try_into_lists(self) -> OrcResult<ListVectorBatch<'a>> {
        ffi::try_into_ListVectorBatch(self.0)
            .map_err(OrcError)
            .map(ListVectorBatch)
    }
}

/// A specialized [ColumnVectorBatch] whose values are known to be structures.
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

/// A specialized [ColumnVectorBatch] whose values are known to be integer-like.
///
/// It is constructed through [`BorrowedColumnVectorBatch::try_into_longs`]
pub struct LongVectorBatch<'a>(&'a ffi::LongVectorBatch);

impl_debug!(LongVectorBatch<'a>, ffi::LongVectorBatch_toString);

impl<'a> LongVectorBatch<'a> {
    pub fn iter(&self) -> LongVectorBatchIterator {
        let data = ffi::LongVectorBatch_get_data(self.0).data();
        let num_elements =
            ffi::get_numElements(ffi::LongVectorBatch_into_ColumnVectorBatch(&self.0));

        LongVectorBatchIterator {
            batch: PhantomData,
            index: 0,
            data,
            num_elements: num_elements
                .try_into()
                .expect("could not convert u64 to isize"),
        }
    }
}

/// Iterator on [LongVectorBatch]
#[derive(Debug, Clone)]
pub struct LongVectorBatchIterator<'a> {
    batch: PhantomData<&'a LongVectorBatch<'a>>,
    index: isize,
    data: *const i64,
    num_elements: isize,
}

impl<'a> Iterator for LongVectorBatchIterator<'a> {
    type Item = i64;

    fn next(&mut self) -> Option<i64> {
        if self.index >= self.num_elements {
            return None;
        }

        // These two should be safe because 'num_elements' should be exactly
        // the number of element in each array, and we checked 'index' is lower
        // than 'num_elements'.
        let datum = unsafe { *self.data.offset(self.index) };

        self.index += 1;

        Some(datum)
    }
}

/// A specialized [ColumnVectorBatch] whose values are known to be floating-point-like
///
/// It is constructed through [`BorrowedColumnVectorBatch::try_into_doubles`]
pub struct DoubleVectorBatch<'a>(&'a ffi::DoubleVectorBatch);

impl_debug!(DoubleVectorBatch<'a>, ffi::DoubleVectorBatch_toString);

impl<'a> DoubleVectorBatch<'a> {
    pub fn iter(&self) -> DoubleVectorBatchIterator {
        let data = ffi::DoubleVectorBatch_get_data(self.0).data();
        let num_elements =
            ffi::get_numElements(ffi::DoubleVectorBatch_into_ColumnVectorBatch(&self.0));

        DoubleVectorBatchIterator {
            batch: PhantomData,
            index: 0,
            data,
            num_elements: num_elements
                .try_into()
                .expect("could not convert u64 to isize"),
        }
    }
}

/// Iterator on [DoubleVectorBatch]
#[derive(Debug, Clone)]
pub struct DoubleVectorBatchIterator<'a> {
    batch: PhantomData<&'a DoubleVectorBatch<'a>>,
    index: isize,
    data: *const f64,
    num_elements: isize,
}

impl<'a> Iterator for DoubleVectorBatchIterator<'a> {
    type Item = f64;

    fn next(&mut self) -> Option<f64> {
        if self.index >= self.num_elements {
            return None;
        }

        // These two should be safe because 'num_elements' should be exactly
        // the number of element in each array, and we checked 'index' is lower
        // than 'num_elements'.
        let datum = unsafe { *self.data.offset(self.index) };

        self.index += 1;

        Some(datum)
    }
}

/// A specialized [ColumnVectorBatch] whose values are known to be string-like.
///
/// It is constructed through [`BorrowedColumnVectorBatch::try_into_strings`]
pub struct StringVectorBatch<'a>(&'a ffi::StringVectorBatch);

impl_debug!(StringVectorBatch<'a>, ffi::StringVectorBatch_toString);

impl<'a> StringVectorBatch<'a> {
    pub fn iter(&self) -> StringVectorBatchIterator {
        let data = ffi::StringVectorBatch_get_data(self.0).data();
        let lengths = ffi::StringVectorBatch_get_length(self.0).data();
        let num_elements =
            ffi::get_numElements(ffi::StringVectorBatch_into_ColumnVectorBatch(&self.0));

        StringVectorBatchIterator {
            batch: PhantomData,
            index: 0,
            data,
            lengths,
            num_elements: num_elements
                .try_into()
                .expect("could not convert u64 to isize"),
        }
    }
}

/// Iterator on [StringVectorBatch]
#[derive(Debug, Clone)]
pub struct StringVectorBatchIterator<'a> {
    batch: PhantomData<&'a StringVectorBatch<'a>>,
    index: isize,
    data: *const *mut c_char, // Pointers to start of strings
    lengths: *const i64,      // Length of each string
    num_elements: isize,
}

impl<'a> Iterator for StringVectorBatchIterator<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        if self.index >= self.num_elements {
            return None;
        }

        // These two should be safe because 'num_elements' should be exactly
        // the number of element in each array, and we checked 'index' is lower
        // than 'num_elements'.
        let datum = unsafe { *self.data.offset(self.index) };
        let length = unsafe { *self.lengths.offset(self.index) };

        self.index += 1;

        let length = length.try_into().expect("could not convert u64 to usize");

        // Should be safe because the length indicates the number of bytes in
        // the string.
        Some(unsafe { std::slice::from_raw_parts(datum as *const u8, length) })
    }
}

/// A specialized [ColumnVectorBatch] whose values are lists of other values
///
/// It is constructed through [`BorrowedColumnVectorBatch::try_into_lists`]
pub struct ListVectorBatch<'a>(&'a ffi::ListVectorBatch);

impl_debug!(ListVectorBatch<'a>, ffi::ListVectorBatch_toString);

impl<'a> ListVectorBatch<'a> {
    /// The flat vector of all elements of all lists
    pub fn elements(&self) -> BorrowedColumnVectorBatch<'a> {
        BorrowedColumnVectorBatch(ffi::ListVectorBatch_get_elements(self.0))
    }

    /// Offset of each ist in the flat vector
    pub fn offsets(&self) -> &'a [i64] {
        let buffer = ffi::ListVectorBatch_get_offsets(self.0).data();
        let num_elements =
            ffi::get_numElements(ffi::ListVectorBatch_into_ColumnVectorBatch(&self.0))
            .try_into().expect("could not convert u64 to usize");

        // Safe because num_elements is exactly the number of elements in the buffer
        unsafe { std::slice::from_raw_parts(buffer, num_elements) }
    }
}
