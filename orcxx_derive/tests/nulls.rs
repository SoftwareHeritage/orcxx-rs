// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

extern crate orcxx;
extern crate orcxx_derive;

use orcxx::deserialize::{CheckableKind, DeserializationError, OrcDeserialize};
use orcxx::reader;
use orcxx_derive::OrcDeserialize;

fn row_reader() -> reader::RowReader {
    let orc_path = "../orcxx/orc/examples/TestOrcFile.testStringAndBinaryStatistics.orc";
    let input_stream = reader::InputStream::from_local_file(orc_path).expect("Could not open .orc");
    let reader = reader::Reader::new(input_stream).expect("Could not read .orc");

    let options = reader::RowReaderOptions::default().include_names(["bytes1", "string1"]);
    reader.row_reader(options).unwrap()
}

#[test]
fn test_all_options() {
    #[derive(OrcDeserialize, Default, Debug, PartialEq)]
    struct Root {
        bytes1: Option<Vec<u8>>,
        string1: Option<String>,
    }

    let mut row_reader = row_reader();
    Root::check_kind(&row_reader.selected_kind()).unwrap();

    let mut rows: Vec<Root> = Vec::new();

    let mut batch = row_reader.row_batch(1024);
    while row_reader.read_into(&mut batch) {
        let new_rows = Root::from_vector_batch(&batch.borrow()).unwrap();
        rows.extend(new_rows);
    }

    assert_eq!(
        rows,
        vec![
            Root {
                bytes1: Some([0, 1, 2, 3, 4].to_vec()),
                string1: Some("foo".to_owned())
            },
            Root {
                bytes1: Some([0, 1, 2, 3].to_vec()),
                string1: Some("bar".to_owned())
            },
            Root {
                bytes1: Some([0, 1, 2, 3, 4, 5].to_vec()),
                string1: None
            },
            Root {
                bytes1: None,
                string1: Some("hi".to_owned())
            }
        ]
    );
}

#[test]
fn test_string_no_option() {
    #[derive(OrcDeserialize, Default, Debug, PartialEq)]
    struct Root {
        bytes1: Option<Vec<u8>>,
        string1: String,
    }

    let mut row_reader = row_reader();
    Root::check_kind(&row_reader.selected_kind()).unwrap();

    let mut batch = row_reader.row_batch(1024);
    assert!(row_reader.read_into(&mut batch));
    assert_eq!(
        Root::from_vector_batch(&batch.borrow()),
        Err(DeserializationError::UnexpectedNull(
            "String column contains nulls".to_owned()
        ))
    );
}
