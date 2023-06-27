// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

extern crate cxx;

#[cxx::bridge]
mod ffi {
    #![allow(dead_code)]

    #[namespace = "orcxx_rs"]
    unsafe extern "C++" {
        include!("cpp-utils.hh");
        include!("orc/OrcFile.hh");

        #[rust_name = "ReaderOptions_new"]
        fn construct() -> UniquePtr<ReaderOptions>;

        #[rust_name = "RowReaderOptions_new"]
        fn construct() -> UniquePtr<RowReaderOptions>;
    }

    #[namespace = "orcxx_rs"]
    unsafe extern "C++" {
        type Int64DataBuffer;

        fn data(&self) -> *const i64;
    }

    #[namespace = "orcxx_rs"]
    unsafe extern "C++" {
        type StringDataBuffer;

        fn data(&self) -> *const *mut c_char;
    }

    #[namespace = "orc"]
    unsafe extern "C++" {
        type InputStream;
        type ReaderOptions;
        type RowReaderOptions;
        type ColumnVectorBatch;
        type StringVectorBatch;
        type StructVectorBatch;

        fn readLocalFile(path: &CxxString) -> Result<UniquePtr<InputStream>>;
    }

    #[namespace = "orc"]
    unsafe extern "C++" {
        type Reader;

        fn createReader(
            inStream: UniquePtr<InputStream>,
            options: &ReaderOptions,
        ) -> UniquePtr<Reader>;

        fn createRowReader(&self, rowReaderOptions: &RowReaderOptions) -> UniquePtr<RowReader>;
    }

    #[namespace = "orc"]
    unsafe extern "C++" {
        type RowReader;

        fn createRowBatch(&self, size: u64) -> UniquePtr<ColumnVectorBatch>;

        fn next(self: Pin<&mut RowReader>, data: Pin<&mut ColumnVectorBatch>) -> bool;
    }

    #[namespace = "orcxx_rs"]
    unsafe extern "C++" {
        type ColumnVectorBatchPtr;

        #[rust_name = "ColumnVectorBatchPtr_make_ptr"]
        fn into(batch_ptr: &ColumnVectorBatchPtr) -> *const ColumnVectorBatch;

        fn get_numElements(vectorBatch: &ColumnVectorBatch) -> u64;

        //#[rust_name = "try_into_StringColumnBatch"]
        //fn ptr_try_into(vectorBatch: UniquePtr<ColumnVectorBatch>) -> Result<UniquePtr<StringVectorBatch>>;
        #[rust_name = "try_into_StringColumnBatch"]
        fn try_into(vectorBatch: &ColumnVectorBatch) -> Result<&StringVectorBatch>;
        #[rust_name = "StringVectorBatch_get_data"]
        fn get_data(vectorBatch: &StringVectorBatch) -> &StringDataBuffer;
        #[rust_name = "StringVectorBatch_get_length"]
        fn get_length(vectorBatch: &StringVectorBatch) -> &Int64DataBuffer;

        #[rust_name = "try_into_StructColumnBatch"]
        fn try_into(vectorBatch: &ColumnVectorBatch) -> Result<&StructVectorBatch>;
        #[rust_name = "StructVectorBatch_get_fields"]
        fn get_fields(vectorBatch: &StructVectorBatch) -> &CxxVector<ColumnVectorBatchPtr>;

    }
}

#[cfg(test)]
mod tests {
    use cxx::let_cxx_string;

    use std::convert::TryInto;
    use std::ffi::CStr;

    use super::*;

    #[test]
    fn nonexistent_file() {
        let_cxx_string!(file_name = "orc/examples/nonexistent.orc");
        assert!(matches!(ffi::readLocalFile(&file_name), Err(_)))
    }

    #[test]
    fn read_file() {
        let_cxx_string!(file_name = "orc/examples/TestOrcFile.test1.orc");
        let input_stream = ffi::readLocalFile(&file_name).expect("Could not read");

        let reader_options = ffi::ReaderOptions_new();
        let reader = ffi::createReader(input_stream, &*reader_options);

        let row_reader_options = ffi::RowReaderOptions_new();
        let mut row_reader = reader.createRowReader(&*row_reader_options);

        let mut batch = row_reader.createRowBatch(1024);

        let mut total_elements = 0;
        let mut all_strings = Vec::new();
        while row_reader.pin_mut().next(batch.pin_mut()) {
            for _ in 0..ffi::get_numElements(&*batch) {
                total_elements += 1;
            }

            let struct_data = ffi::StructVectorBatch_get_fields(
                &*ffi::try_into_StructColumnBatch(&*batch)
                    .expect("could not cast ColumnVectorBatch to StructDataBuffer"),
            );

            for sub_batch in struct_data {
                match ffi::try_into_StringColumnBatch(unsafe {
                    &*ffi::ColumnVectorBatchPtr_make_ptr(sub_batch)
                }) {
                    Ok(sub_batch) => {
                        let data = ffi::StringVectorBatch_get_data(sub_batch);
                        let lengths = ffi::StringVectorBatch_get_length(sub_batch);
                        let num_elements: usize = ffi::get_numElements(&*batch)
                            .try_into()
                            .expect("could not convert u64 to usize");
                        unsafe {
                            for (&s, &length) in std::iter::zip(
                                std::slice::from_raw_parts(data.data(), num_elements),
                                std::slice::from_raw_parts(lengths.data(), num_elements),
                            ) {
                                all_strings.push(std::str::from_utf8(std::slice::from_raw_parts(
                                    s as *const u8,
                                    length.try_into().expect("could not convert u64 to usize"),
                                )))
                            }
                        }
                    }
                    Err(e) => println!("failed to cast to StringDataBuffer: {:?}", e),
                }
            }
        }

        assert_eq!(total_elements, 2);
        assert_eq!(
            all_strings,
            vec![Ok("\0\u{1}\u{2}\u{3}\u{4}"), Ok(""), Ok("hi"), Ok("bye")]
        );
    }
}
