// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

extern crate orcxx;
extern crate orcxx_derive;

use std::convert::TryInto;

use orcxx::deserialize::{CheckableKind, OrcDeserialize};
use orcxx::reader;
use orcxx::row_iterator::RowIterator;
use orcxx_derive::OrcDeserialize;

fn get_row_reader() -> reader::RowReader {
    let orc_path = "../orc/examples/TestOrcFile.test1.orc";
    let input_stream = reader::InputStream::from_local_file(orc_path).expect("Could not open .orc");
    let reader = reader::Reader::new(input_stream).expect("Could not read .orc");

    let options = reader::RowReaderOptions::default().include_names([
        "boolean1", "byte1", "short1", "int1", "long1", "float1", "double1", "bytes1", "string1",
        "list",
    ]);
    reader.row_reader(options).unwrap()
}

#[derive(OrcDeserialize, Clone, Default, Debug, PartialEq)]
struct Test1Option {
    boolean1: Option<bool>,
    byte1: Option<i8>,
    short1: Option<i16>,
    int1: Option<i32>,
    long1: Option<i64>,
    float1: Option<f32>,
    double1: Option<f64>,
    bytes1: Option<Vec<u8>>,
    string1: Option<String>,
    list: Option<Vec<Option<Test1ItemOption>>>,
}

#[derive(OrcDeserialize, Clone, Default, Debug, PartialEq)]
struct Test1ItemOption {
    int1: Option<i32>,
    string1: Option<String>,
}

fn expected_rows_options() -> Vec<Test1Option> {
    vec![
        Test1Option {
            boolean1: Some(false),
            byte1: Some(1),
            short1: Some(1024),
            int1: Some(65536),
            long1: Some(9223372036854775807),
            float1: Some(1.0),
            double1: Some(-15.0),
            bytes1: Some(vec![0, 1, 2, 3, 4]),
            string1: Some("hi".to_owned()),
            list: Some(vec![
                Some(Test1ItemOption {
                    int1: Some(3),
                    string1: Some("good".to_owned()),
                }),
                Some(Test1ItemOption {
                    int1: Some(4),
                    string1: Some("bad".to_owned()),
                }),
            ]),
        },
        Test1Option {
            boolean1: Some(true),
            byte1: Some(100),
            short1: Some(2048),
            int1: Some(65536),
            long1: Some(9223372036854775807),
            float1: Some(2.0),
            double1: Some(-5.0),
            bytes1: Some(vec![]),
            string1: Some("bye".to_owned()),
            list: Some(vec![
                Some(Test1ItemOption {
                    int1: Some(100000000),
                    string1: Some("cat".to_owned()),
                }),
                Some(Test1ItemOption {
                    int1: Some(-100000),
                    string1: Some("in".to_owned()),
                }),
                Some(Test1ItemOption {
                    int1: Some(1234),
                    string1: Some("hat".to_owned()),
                }),
            ]),
        },
    ]
}

/// Tests `Option<Test1Option>::from_vector_batch()`
#[test]
fn test1_inner_option_outer_option() {
    let mut row_reader = get_row_reader();
    Test1Option::check_kind(&row_reader.selected_kind()).unwrap();

    let mut rows: Vec<Option<Test1Option>> = Vec::new();

    let mut batch = row_reader.row_batch(1024);
    while row_reader.read_into(&mut batch) {
        let new_rows = Option::<Test1Option>::from_vector_batch(&batch.borrow()).unwrap();
        rows.extend(new_rows);
    }

    // Make sure both APIs are consistent
    let row_reader = get_row_reader();
    assert_eq!(
        rows,
        RowIterator::<Option<Test1Option>>::new(row_reader, 10.try_into().unwrap())
            .collect::<Vec<_>>()
    );

    assert_eq!(
        rows,
        expected_rows_options()
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>()
    );
}

/// Tests `Test1Option::from_vector_batch()`
#[test]
fn test1_inner_option_outer_nooption() {
    let mut row_reader = get_row_reader();
    Test1Option::check_kind(&row_reader.selected_kind()).unwrap();

    let mut rows: Vec<Test1Option> = Vec::new();

    let mut batch = row_reader.row_batch(1024);
    while row_reader.read_into(&mut batch) {
        let new_rows = Test1Option::from_vector_batch(&batch.borrow()).unwrap();
        rows.extend(new_rows);
    }

    // Make sure both APIs are consistent
    let row_reader = get_row_reader();
    assert_eq!(
        rows,
        RowIterator::<Test1Option>::new(row_reader, 10.try_into().unwrap()).collect::<Vec<_>>()
    );

    assert_eq!(rows, expected_rows_options());
}

#[derive(OrcDeserialize, Clone, Default, Debug, PartialEq)]
struct Test1NoOption {
    boolean1: bool,
    byte1: i8,
    short1: i16,
    int1: i32,
    long1: i64,
    float1: f32,
    double1: f64,
    bytes1: Vec<u8>,
    string1: String,
    list: Vec<Test1ItemNoOption>,
}

#[derive(OrcDeserialize, Clone, Default, Debug, PartialEq)]
struct Test1ItemNoOption {
    int1: i32,
    string1: String,
}

fn expected_rows_nooptions() -> Vec<Test1NoOption> {
    vec![
        Test1NoOption {
            boolean1: false,
            byte1: 1,
            short1: 1024,
            int1: 65536,
            long1: 9223372036854775807,
            float1: 1.0,
            double1: -15.0,
            bytes1: vec![0, 1, 2, 3, 4],
            string1: "hi".to_owned(),
            list: vec![
                Test1ItemNoOption {
                    int1: 3,
                    string1: "good".to_owned(),
                },
                Test1ItemNoOption {
                    int1: 4,
                    string1: "bad".to_owned(),
                },
            ],
        },
        Test1NoOption {
            boolean1: true,
            byte1: 100,
            short1: 2048,
            int1: 65536,
            long1: 9223372036854775807,
            float1: 2.0,
            double1: -5.0,
            bytes1: vec![],
            string1: "bye".to_owned(),
            list: vec![
                Test1ItemNoOption {
                    int1: 100000000,
                    string1: "cat".to_owned(),
                },
                Test1ItemNoOption {
                    int1: -100000,
                    string1: "in".to_owned(),
                },
                Test1ItemNoOption {
                    int1: 1234,
                    string1: "hat".to_owned(),
                },
            ],
        },
    ]
}

/// Tests `Option<Test1NoOption>::from_vector_batch()`
#[test]
fn test1_inner_nooption_outer_option() {
    let mut row_reader = get_row_reader();

    Test1NoOption::check_kind(&row_reader.selected_kind()).unwrap();

    let mut rows: Vec<Option<Test1NoOption>> = Vec::new();

    let mut batch = row_reader.row_batch(1024);
    while row_reader.read_into(&mut batch) {
        let new_rows = Option::<Test1NoOption>::from_vector_batch(&batch.borrow()).unwrap();
        rows.extend(new_rows);
    }

    // Make sure both APIs are consistent
    let row_reader = get_row_reader();
    assert_eq!(
        rows,
        RowIterator::<Option<Test1NoOption>>::new(row_reader, 10.try_into().unwrap())
            .collect::<Vec<_>>()
    );

    assert_eq!(
        rows,
        expected_rows_nooptions()
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>()
    );
}

/// Tests `Test1NoOption::from_vector_batch()`
#[test]
fn test1_inner_nooption_outer_nooption() {
    let mut row_reader = get_row_reader();
    Test1NoOption::check_kind(&row_reader.selected_kind()).unwrap();

    let mut rows: Vec<Test1NoOption> = Vec::new();

    let mut batch = row_reader.row_batch(1024);
    while row_reader.read_into(&mut batch) {
        let new_rows = Test1NoOption::from_vector_batch(&batch.borrow()).unwrap();
        rows.extend(new_rows);
    }

    // Make sure both APIs are consistent
    let row_reader = get_row_reader();
    assert_eq!(
        rows,
        RowIterator::<Test1NoOption>::new(row_reader, 10.try_into().unwrap()).collect::<Vec<_>>()
    );

    assert_eq!(rows, expected_rows_nooptions());
}
