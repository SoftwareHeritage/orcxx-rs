// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

/// Converts ORC files to successive JSON objects
extern crate orcxx;

use std::io::Write;
use std::{env, io, process};

use orcxx::reader;
use orcxx::structured_reader::StructuredRowReader;
use orcxx::to_json::columntree_to_json_rows;

fn to_json(orc_path: &str) {
    let input_stream = reader::InputStream::from_local_file(orc_path).expect("Could not open .orc");
    let reader = reader::Reader::new(input_stream).expect("Could not read .orc");

    let mut row_reader = reader
        .row_reader(reader::RowReaderOptions::default())
        .unwrap();

    let mut structured_row_reader = StructuredRowReader::new(&mut row_reader, 1024);

    while let Some(columns) = structured_row_reader.next() {
        for object in columntree_to_json_rows(columns) {
            println!("{}", json::stringify_pretty(object, 4));
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.as_slice() {
        [_, path] => to_json(path),
        _ => {
            io::stderr()
                .write_all(b"Syntax: <path>\n\nReads an ORC file and prints it as JSON objects.\n")
                .unwrap();
            process::exit(1);
        }
    }
}
