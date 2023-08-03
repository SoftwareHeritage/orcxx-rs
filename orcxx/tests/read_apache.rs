// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

#![allow(non_snake_case)]

#[cfg(not(feature = "json"))]
compile_error!("Feature 'json' must be enabled for this test.");

/// Tests against `.orc` and `.jsn.gz` in the official test suite (`orc/examples/`)
extern crate flate2;
extern crate json;
extern crate orcxx;
extern crate pretty_assertions;

use std::fs;
use std::io::Read;

use pretty_assertions::assert_eq;

use orcxx::structured_reader::StructuredRowReader;
use orcxx::to_json::columntree_to_json_rows;
use orcxx::*;

/// Checks parsing a `.orc` file produces the expected result in the `.jsn.gz` path
fn test_expected_file(orc_path: &str, jsn_gz_path: &str) {
    let input_stream = reader::InputStream::from_local_file(orc_path).expect("Could not open .orc");
    let reader = reader::Reader::new(input_stream).expect("Could not read .orc");

    let mut row_reader = reader
        .row_reader(reader::RowReaderOptions::default())
        .unwrap();

    let mut structured_row_reader = StructuredRowReader::new(&mut row_reader, 1024);

    let mut objects = Vec::new();

    while let Some(columns) = structured_row_reader.next() {
        objects.extend(columntree_to_json_rows(columns));
    }

    let mut expected_json = String::new();
    flate2::read::GzDecoder::new(&fs::File::open(jsn_gz_path).expect("Could not open .jsn.gz"))
        .read_to_string(&mut expected_json)
        .expect("Could not read .jsn.gz");

    let objects_count = objects.len();

    // Reencode the input to normalize it
    let expected_lines = expected_json
        .split("\n")
        .filter(|line| line.len() > 0)
        .map(|line| json::parse(line).expect("Could not parse line in .jsn.gz"))
        .map(|o| json::stringify_pretty(o, 4))
        .collect::<Vec<_>>()
        .join("\n");

    let lines = objects
        .into_iter()
        .map(|o| json::stringify_pretty(o, 4))
        .collect::<Vec<_>>()
        .join("\n");

    if lines.len() < 10000 {
        assert_eq!(lines, expected_lines);
    } else {
        // pretty_assertions consumes too much RAM and CPU on large diffs,
        // and it's unreadable anyway
        assert!(lines == expected_lines);
    }

    assert_eq!(reader.row_count(), objects_count as u64);
}

macro_rules! test_apache_file {
    ($name:literal) => {
        test_expected_file(
            concat!("../orc/examples/", $name, ".orc"),
            concat!("../orc/examples/expected/", $name, ".jsn.gz"),
        )
    };
}

#[test]
fn columnProjection() {
    test_apache_file!("TestOrcFile.columnProjection");
}
#[test]
fn emptyFile() {
    test_apache_file!("TestOrcFile.emptyFile");
}
#[test]
#[ignore] // Differs on representation of some Decimals
fn metaData() {
    test_apache_file!("TestOrcFile.metaData");
}
#[test]
fn test1() {
    test_apache_file!("TestOrcFile.test1");
}
#[test]
fn testDate1900() {
    test_apache_file!("TestOrcFile.testDate1900");
}
#[test]
fn testDate2038() {
    test_apache_file!("TestOrcFile.testDate2038");
}
#[test]
fn testMemoryManagementV11() {
    test_apache_file!("TestOrcFile.testMemoryManagementV11");
}
#[test]
fn testMemoryManagementV12() {
    test_apache_file!("TestOrcFile.testMemoryManagementV12");
}
#[test]
fn testPredicatePushdown() {
    test_apache_file!("TestOrcFile.testPredicatePushdown");
}
#[test]
#[ignore] // Crashes the process
fn testSeek() {
    test_apache_file!("TestOrcFile.testSeek");
}
#[test]
fn testSnappy() {
    test_apache_file!("TestOrcFile.testSnappy");
}
#[test]
fn testStringAndBinaryStatistics() {
    test_apache_file!("TestOrcFile.testStringAndBinaryStatistics");
}
#[test]
fn testStripeLevelStats() {
    test_apache_file!("TestOrcFile.testStripeLevelStats");
}
#[test]
fn testTimestamp() {
    test_apache_file!("TestOrcFile.testTimestamp");
}
#[test]
#[ignore] // Unions are not supported yet
fn testUnionAndTimestamp() {
    test_apache_file!("TestOrcFile.testUnionAndTimestamp");
}
#[test]
fn testWithoutIndex() {
    test_apache_file!("TestOrcFile.testWithoutIndex");
}
#[test]
fn testLz4() {
    test_apache_file!("TestVectorOrcFile.testLz4");
}
#[test]
fn testLzo() {
    test_apache_file!("TestVectorOrcFile.testLzo");
}
#[test]
#[ignore] // Differs on representation of some Decimals
fn decimal() {
    test_apache_file!("decimal");
}
#[test]
#[ignore] // Too slow
fn zlib() {
    test_apache_file!("demo-12-zlib");
}
#[test]
#[ignore] // Overflows the JSON library
fn nulls_at_end_snappy() {
    test_apache_file!("nulls-at-end-snappy");
}
#[test]
fn orc_11_format() {
    test_apache_file!("orc-file-11-format");
}
#[test]
fn orc_index_int_string() {
    test_apache_file!("orc_index_int_string");
}
#[test]
#[ignore] // Differs on representation of some Decimals
fn orc_split_elim() {
    test_apache_file!("orc_split_elim");
}
#[test]
#[ignore] // Differs on representation of some Decimals
fn orc_split_elim_cpp() {
    test_apache_file!("orc_split_elim_cpp");
}
#[test]
#[ignore] // Differs on representation of some Decimals
fn orc_split_elim_new() {
    test_apache_file!("orc_split_elim_new");
}
#[test]
#[ignore] // Differs on representation of some Decimals
fn over1k_bloom() {
    test_apache_file!("over1k_bloom");
}
