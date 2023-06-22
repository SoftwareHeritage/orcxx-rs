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

        #[rust_name = "ReaderOptions_new"]
        fn construct() -> UniquePtr<ReaderOptions>;
    }

    #[namespace = "orc"]
    unsafe extern "C++" {
        include!("orc/OrcFile.hh");

        type InputStream;
        type ReaderOptions;
        type Reader;

        fn readLocalFile(path: &CxxString) -> Result<UniquePtr<InputStream>>;
        fn createReader(
            inStream: UniquePtr<InputStream>,
            options: &ReaderOptions,
        ) -> UniquePtr<Reader>;
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
        let options = ffi::ReaderOptions_new();
        ffi::createReader(input_stream, &*options);
    }
}
