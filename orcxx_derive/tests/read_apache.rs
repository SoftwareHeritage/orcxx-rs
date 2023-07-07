// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

extern crate orcxx;
extern crate orcxx_derive;

use orcxx::deserialize::OrcDeserializable;
use orcxx::reader;
use orcxx_derive::OrcDeserialize;

#[derive(OrcDeserialize, Default, Debug, PartialEq, Eq)]
struct Test1 {
    long1: Option<i64>,
    bytes1: Option<Vec<u8>>,
    string1: Option<String>,
}

#[test]
fn test1_option() {
    let orc_path = "../orc/examples/TestOrcFile.test1.orc";
    let input_stream = reader::InputStream::from_local_file(orc_path).expect("Could not open .orc");
    let reader = reader::Reader::new(input_stream).expect("Could not read .orc");

    let options = reader::RowReaderOptions::default().include_names(["long1", "bytes1", "string1"]);
    let mut row_reader = reader.row_reader(options);
    println!("{:?}", row_reader.selected_kind());

    let mut rows: Vec<Option<Test1>> = Vec::new();

    let mut batch = row_reader.row_batch(1024);
    while row_reader.read_into(&mut batch) {
        let new_rows = Test1::options_from_vector_batch(&batch.borrow()).unwrap();
        rows.extend(new_rows);
    }

    assert_eq!(
        rows,
        vec![
            Some(Test1 {
                long1: Some(9223372036854775807),
                bytes1: Some(vec![0, 1, 2, 3, 4]),
                string1: Some("hi".to_owned()),
            }),
            Some(Test1 {
                long1: Some(9223372036854775807),
                bytes1: Some(vec![]),
                string1: Some("bye".to_owned()),
            })
        ]
    );
}
