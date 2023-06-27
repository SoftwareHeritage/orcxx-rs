// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

use std::convert::TryInto;
use std::marker::PhantomData;
use std::os::raw::c_char;

use cxx::UniquePtr;

use utils::{OrcError, OrcResult};

#[cxx::bridge]
pub(crate) mod ffi {
    // Reimport types from other modules
    #[namespace = "orcxx_rs"]
    unsafe extern "C++" {
        type Int64DataBuffer = crate::memorypool::ffi::Int64DataBuffer;
        type StringDataBuffer = crate::memorypool::ffi::StringDataBuffer;
    }

    #[namespace = "orc"]
    unsafe extern "C++" {
        include!("cpp-utils.hh");
        include!("orc/Vector.hh");

        type ColumnVectorBatch;
        type StringVectorBatch;
        type StructVectorBatch;
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

        #[rust_name = "StringVectorBatch_get_data"]
        fn get_data(vectorBatch: &StringVectorBatch) -> &StringDataBuffer;
        #[rust_name = "StringVectorBatch_get_length"]
        fn get_length(vectorBatch: &StringVectorBatch) -> &Int64DataBuffer;

        #[rust_name = "StructVectorBatch_get_fields"]
        fn get_fields(vectorBatch: &StructVectorBatch) -> &CxxVector<ColumnVectorBatchPtr>;
    }

    #[namespace = "orcxx_rs::utils"]
    unsafe extern "C++" {
        #[rust_name = "try_into_StringVectorBatch"]
        fn try_into(vectorBatch: &ColumnVectorBatch) -> Result<&StringVectorBatch>;

        #[rust_name = "try_into_StructVectorBatch"]
        fn try_into(vectorBatch: &ColumnVectorBatch) -> Result<&StructVectorBatch>;

        #[rust_name = "StringVectorBatch_into_ColumnVectorBatch"]
        fn try_into(vectorBatch: &StringVectorBatch) -> &ColumnVectorBatch;
    }
}

/// A column (or set of column) of a stripe, with values of unknown type.
pub struct OwnedColumnVectorBatch(pub(crate) UniquePtr<ffi::ColumnVectorBatch>);

impl OwnedColumnVectorBatch {
    pub fn num_elements(&self) -> u64 {
        ffi::get_numElements(&self.0)
    }

    pub fn as_structs(&self) -> OrcResult<StructVectorBatch> {
        ffi::try_into_StructVectorBatch(&self.0)
            .map_err(OrcError)
            .map(StructVectorBatch)
    }
}

/// A column (or set of column) of a stripe, with values of unknown type.
pub struct BorrowedColumnVectorBatch<'a>(&'a ffi::ColumnVectorBatch);

impl<'a> BorrowedColumnVectorBatch<'a> {
    pub fn num_elements(&self) -> u64 {
        ffi::get_numElements(&self.0)
    }

    pub fn as_structs(&self) -> OrcResult<StructVectorBatch<'a>> {
        ffi::try_into_StructVectorBatch(&self.0)
            .map_err(OrcError)
            .map(StructVectorBatch)
    }

    pub fn as_strings(&self) -> OrcResult<StringVectorBatch<'a>> {
        ffi::try_into_StringVectorBatch(&self.0)
            .map_err(OrcError)
            .map(StringVectorBatch)
    }
}

/// A specialized [ColumnVectorBatch] whose values are known to be structures.
///
/// It is constructed through [`OwnedColumnVectorBatch::as_structs`]
/// or  [`BorrowedColumnVectorBatch::as_structs`]
pub struct StructVectorBatch<'a>(&'a ffi::StructVectorBatch);

impl<'a> StructVectorBatch<'a> {
    pub fn fields(&self) -> Vec<BorrowedColumnVectorBatch> {
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

/// A specialized [ColumnVectorBatch] whose values are known to be string-like.
///
/// It is constructed through [`OwnedColumnVectorBatch::as_strings`]
/// or  [`BorrowedColumnVectorBatch::as_strings`]
pub struct StringVectorBatch<'a>(&'a ffi::StringVectorBatch);

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
