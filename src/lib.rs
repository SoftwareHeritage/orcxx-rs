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

    #[namespace = "orc"]
    unsafe extern "C++" {
        type InputStream;
        type ReaderOptions;
        type RowReaderOptions;
        type ColumnVectorBatch;

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
        fn get_numElements(vectorBatch: &ColumnVectorBatch) -> u64;
    }
}

#[cfg(test)]
mod tests {
    use cxx::let_cxx_string;

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
        while row_reader.pin_mut().next(batch.pin_mut()) {
            for _ in 0..ffi::get_numElements(&*batch) {
                total_elements+= 1;
            }
        }

        assert_eq!(total_elements, 2);
    }
}
