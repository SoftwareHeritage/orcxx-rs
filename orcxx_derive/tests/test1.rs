// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

extern crate orcxx;
extern crate orcxx_derive;

use std::convert::TryInto;

use orcxx::deserialize::{CheckableKind, OrcDeserialize, OrcStruct};
use orcxx::reader;
use orcxx::row_iterator::RowIterator;
use orcxx_derive::OrcDeserialize;

fn get_reader() -> reader::Reader {
    let orc_path = "../orcxx/orc/examples/TestOrcFile.test1.orc";
    let input_stream = reader::InputStream::from_local_file(orc_path).expect("Could not open .orc");
    reader::Reader::new(input_stream).expect("Could not read .orc")
}

fn get_row_reader_options() -> reader::RowReaderOptions {
    reader::RowReaderOptions::default().include_names([
        "boolean1", "byte1", "short1", "int1", "long1", "float1", "double1", "bytes1", "string1",
        "list",
    ])
}

fn get_row_reader() -> reader::RowReader {
    let reader = get_reader();

    reader.row_reader(&get_row_reader_options()).unwrap()
}

fn test_with_batch_size<
    const BATCH_SIZE: u64,
    T: CheckableKind + OrcDeserialize + OrcStruct + Clone + PartialEq + std::fmt::Debug,
>(
    expected_rows: Vec<T>,
) {
    let reader = get_reader();
    let mut row_reader = get_row_reader();

    T::check_kind(&row_reader.selected_kind()).unwrap();

    let mut rows: Vec<T> = Vec::new();

    let mut batch = row_reader.row_batch(BATCH_SIZE);
    while row_reader.read_into(&mut batch) {
        let new_rows = T::from_vector_batch(&batch.borrow()).unwrap();
        rows.extend(new_rows);
    }

    assert_eq!(
        expected_rows, rows,
        "Unexpected rows when using from_vector_batch API"
    );

    assert_eq!(
        expected_rows,
        RowIterator::<T>::new_with_options(
            &reader,
            BATCH_SIZE.try_into().unwrap(),
            &get_row_reader_options()
        )
        .unwrap()
        .collect::<Vec<_>>(),
        "Inconsistent set of rows when using RowIterator"
    );

    assert_eq!(
        expected_rows,
        RowIterator::<T>::new(&reader, BATCH_SIZE.try_into().unwrap())
            .unwrap()
            .collect::<Vec<_>>(),
        "Inconsistent set of rows when RowIterator constructed with default options"
    );

    // Test manual iteration
    let mut iter = RowIterator::<T>::new(&reader, BATCH_SIZE.try_into().unwrap()).unwrap();
    assert_eq!(iter.len(), expected_rows.len());
    for (i, expected_row) in expected_rows.iter().enumerate() {
        assert_eq!(
            expected_rows.len() - i,
            iter.len(),
            "Number of rows changed halfway (at row {i})"
        );
        assert_eq!(
            iter.next().as_ref(),
            Some(expected_row),
            "Inconsistent row #{i}"
        );
    }
    assert_eq!(iter.next(), None, "Too many rows");

    // Test manual iteration backward
    for (i, expected_row) in expected_rows.iter().rev().enumerate() {
        assert_eq!(
            i,
            iter.len(),
            "Number of rows changed halfway (at row {i})"
        );
        assert_eq!(
            iter.next_back().as_ref(),
            Some(expected_row),
            "Inconsistent row #{}",
            expected_rows.len() - i - 1
        );
    }
    assert_eq!(iter.next_back(), None, "Too many rows backward");

    // Go halfway then back
    assert_eq!(iter.next().as_ref(), Some(&expected_rows[0]));
    assert_eq!(iter.next_back().as_ref(), Some(&expected_rows[0]));
    assert_eq!(iter.next_back().as_ref(), None);

    // Go full forward, rewind halfway, then forward again
    for expected_row in expected_rows.iter() {
        assert_eq!(iter.next().as_ref(), Some(expected_row));
    }
    assert_eq!(
        iter.next_back().as_ref(),
        Some(&expected_rows[expected_rows.len() - 1])
    );
    assert_eq!(
        iter.next().as_ref(),
        Some(&expected_rows[expected_rows.len() - 1])
    );
    assert_eq!(iter.next().as_ref(), None);
}

fn test<T: CheckableKind + OrcDeserialize + OrcStruct + Clone + PartialEq + std::fmt::Debug>(
    expected_rows: Vec<T>,
) {
    // Using a const generic so it is more obvious on stack traces which value
    // is causing a test failure
    test_with_batch_size::<1, T>(expected_rows.clone());
    test_with_batch_size::<2, T>(expected_rows.clone());
    test_with_batch_size::<3, T>(expected_rows.clone());
    test_with_batch_size::<10, T>(expected_rows.clone());
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
    test::<Option<Test1Option>>(
        expected_rows_options()
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>(),
    );
}

/// Tests `Test1Option::from_vector_batch()`
#[test]
fn test1_inner_option_outer_nooption() {
    test::<Test1Option>(expected_rows_options());
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
    test::<Option<Test1NoOption>>(
        expected_rows_nooptions()
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>(),
    );
}

/// Tests `Test1NoOption::from_vector_batch()`
#[test]
fn test1_inner_nooption_outer_nooption() {
    test::<Test1NoOption>(expected_rows_nooptions());
}
