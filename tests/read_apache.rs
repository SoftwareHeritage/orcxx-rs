// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

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


#[test]
fn read_file() {
    let input_stream = reader::InputStream::from_local_file("orc/examples/TestOrcFile.test1.orc")
        .expect("Could not open .orc");
    let reader = reader::Reader::new(input_stream).expect("Could not read .orc");

    let mut row_reader = reader.row_reader(reader::RowReaderOptions::default());

    let mut structured_row_reader = StructuredRowReader::new(&mut row_reader, 1024);

    let mut objects = Vec::new();

    while let Some(columns) = structured_row_reader.next() {
        objects.extend(columntree_to_json_rows(&columns));
    }

    let mut expected_json = String::new();
    flate2::read::GzDecoder::new(
        &fs::File::open("orc/examples/expected/TestOrcFile.test1.jsn.gz")
            .expect("Could not open .jsn.gz"),
    )
    .read_to_string(&mut expected_json)
    .expect("Could not read .jsn.gz");

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

    assert_eq!(lines, expected_lines);
}
