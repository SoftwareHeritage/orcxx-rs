// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

extern crate cxx;

pub mod memorypool;
pub mod reader;
pub mod vector;

#[cfg(test)]
mod tests {
    use cxx::let_cxx_string;

    use std::convert::TryInto;

    use super::*;

    #[test]
    fn nonexistent_file() {
        let_cxx_string!(file_name = "orc/examples/nonexistent.orc");
        assert!(matches!(reader::ffi::readLocalFile(&file_name), Err(_)))
    }

    #[test]
    fn read_file() {
        let_cxx_string!(file_name = "orc/examples/TestOrcFile.test1.orc");
        let input_stream = reader::ffi::readLocalFile(&file_name).expect("Could not read");

        let reader_options = reader::ffi::ReaderOptions_new();
        let reader = reader::ffi::createReader(input_stream, &*reader_options);

        let row_reader_options = reader::ffi::RowReaderOptions_new();
        let mut row_reader = reader.createRowReader(&*row_reader_options);

        let mut batch = row_reader.createRowBatch(1024);

        let mut total_elements = 0;
        let mut all_strings = Vec::new();
        while row_reader.pin_mut().next(batch.pin_mut()) {
            for _ in 0..vector::ffi::get_numElements(&*batch) {
                total_elements += 1;
            }

            let struct_data = vector::ffi::StructVectorBatch_get_fields(
                &*vector::ffi::try_into_StructColumnBatch(&*batch)
                    .expect("could not cast ColumnVectorBatch to StructDataBuffer"),
            );

            for sub_batch in struct_data {
                match vector::ffi::try_into_StringColumnBatch(unsafe {
                    &*vector::ffi::ColumnVectorBatchPtr_make_ptr(sub_batch)
                }) {
                    Ok(sub_batch) => {
                        let data = vector::ffi::StringVectorBatch_get_data(sub_batch);
                        let lengths = vector::ffi::StringVectorBatch_get_length(sub_batch);
                        let num_elements: usize = vector::ffi::get_numElements(&*batch)
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
