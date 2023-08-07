// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

extern crate orcxx;

use orcxx::reader;

#[test]
fn test_string_bytes_and_ranges_without_nulls() {
    let input_stream = reader::InputStream::from_local_file("orc/examples/TestOrcFile.test1.orc")
        .expect("Could not read");
    let reader = reader::Reader::new(input_stream).expect("Could not create reader");

    let mut row_reader = reader
        .row_reader(reader::RowReaderOptions::default().include_names(["bytes1", "string1"]))
        .unwrap();

    let mut batch = row_reader.row_batch(1024);

    assert!(row_reader.read_into(&mut batch));

    let struct_vector = batch
        .borrow()
        .try_into_structs()
        .expect("could not cast ColumnVectorBatch to StructDataBuffer");
    let vectors = struct_vector.fields();
    assert_eq!(vectors.len(), 2);

    let bytes1_vector = vectors[0].try_into_strings().unwrap();
    let string1_vector = vectors[1].try_into_strings().unwrap();
    assert_eq!(bytes1_vector.bytes(), [0, 1, 2, 3, 4]);
    assert_eq!(string1_vector.bytes(), b"hibye");
    assert_eq!(bytes1_vector.ranges(), [Some(0..5), Some(5..5)]);
    assert_eq!(string1_vector.ranges(), [Some(0..2), Some(2..5)]);
}

#[test]
fn test_string_bytes_and_ranges_with_nulls() {
    let input_stream = reader::InputStream::from_local_file(
        "orc/examples/TestOrcFile.testStringAndBinaryStatistics.orc",
    )
    .expect("Could not read");
    let reader = reader::Reader::new(input_stream).expect("Could not create reader");

    let mut row_reader = reader
        .row_reader(reader::RowReaderOptions::default().include_names(["bytes1", "string1"]))
        .unwrap();

    let mut batch = row_reader.row_batch(1024);

    assert!(row_reader.read_into(&mut batch));

    let struct_vector = batch
        .borrow()
        .try_into_structs()
        .expect("could not cast ColumnVectorBatch to StructDataBuffer");
    let vectors = struct_vector.fields();
    assert_eq!(vectors.len(), 2);

    let bytes1_vector = vectors[0].try_into_strings().unwrap();
    let string1_vector = vectors[1].try_into_strings().unwrap();
    assert_eq!(
        bytes1_vector.bytes(),
        [0, 1, 2, 3, 4, 0, 1, 2, 3, 0, 1, 2, 3, 4, 5]
    );
    assert_eq!(string1_vector.bytes(), b"foobarhi");
    assert_eq!(
        bytes1_vector.ranges(),
        [Some(0..5), Some(5..9), Some(9..15), None]
    );
    assert_eq!(
        string1_vector.ranges(),
        [Some(0..3), Some(3..6), None, Some(6..8)]
    );
}
