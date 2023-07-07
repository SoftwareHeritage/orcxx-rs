// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

extern crate orcxx;
extern crate orcxx_derive;

use orcxx::deserialize::CheckableKind;
use orcxx::reader;
use orcxx_derive::OrcDeserialize;

#[derive(OrcDeserialize, Default, Debug, PartialEq, Eq)]
struct Test1IncorrectOrder {
    long1: Option<i64>,
    string1: Option<String>,
    bytes1: Option<Vec<u8>>,
}

/// Tests when the order of fields in the file is not consistent with the struct's
/// (string1 and bytes1 are swapped)
#[test]
fn incorrect_order() {
    let orc_path = "../orc/examples/TestOrcFile.test1.orc";
    let input_stream = reader::InputStream::from_local_file(orc_path).expect("Could not open .orc");
    let reader = reader::Reader::new(input_stream).expect("Could not read .orc");

    let options = reader::RowReaderOptions::default().include_names(["long1", "string1", "bytes1"]);
    let row_reader = reader.row_reader(options).unwrap();
    assert_eq!(
        Test1IncorrectOrder::check_kind(&row_reader.selected_kind()),
        Err("Test1IncorrectOrder cannot be decoded:\n\tField #1 must be called string1, not bytes1\n\tField #2 must be called bytes1, not string1".to_string()));
}

#[derive(OrcDeserialize, Default, Debug, PartialEq, Eq)]
struct Test1IncorrectType {
    long1: Option<i64>,
    bytes1: Option<String>,
}

/// Tests when the order of fields in the file is not consistent with the struct's
/// (string1 and bytes1 are swapped)
#[test]
fn incorrect_type() {
    let orc_path = "../orc/examples/TestOrcFile.test1.orc";
    let input_stream = reader::InputStream::from_local_file(orc_path).expect("Could not open .orc");
    let reader = reader::Reader::new(input_stream).expect("Could not read .orc");

    let options = reader::RowReaderOptions::default().include_names(["long1", "bytes1"]);
    let row_reader = reader.row_reader(options).unwrap();
    assert_eq!(
        Test1IncorrectType::check_kind(&row_reader.selected_kind()),
        Err("Test1IncorrectType cannot be decoded:\n\tField bytes1 cannot be decoded: String must be decoded from ORC String, not ORC Binary".to_string()));
}
