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
use std::iter;

use pretty_assertions::assert_eq;

use json::JsonValue;

use orcxx::structured_reader::{ColumnTree, StructuredRowReader};
use orcxx::*;

fn columntree_to_json_rows<'a>(tree: &ColumnTree<'a>) -> Vec<JsonValue> {
    match tree {
        ColumnTree::Boolean(column) => column.iter().map(|b| JsonValue::Boolean(b != 0)).collect(),
        ColumnTree::Byte(column)
        | ColumnTree::Short(column)
        | ColumnTree::Int(column)
        | ColumnTree::Long(column) => column.iter().map(|b| JsonValue::Number(b.into())).collect(),
        ColumnTree::Float(column) | ColumnTree::Double(column) => {
            column.iter().map(|b| JsonValue::Number(b.into())).collect()
        }
        ColumnTree::String(column) => column
            .iter()
            .map(|s| JsonValue::String(String::from_utf8_lossy(s).into_owned()))
            .collect(),
        ColumnTree::Binary(column) => column
            .iter()
            .map(|s| {
                JsonValue::Array(
                    s.into_iter()
                        .map(|&byte| JsonValue::Number(byte.into()))
                        .collect(),
                )
            })
            .collect(),
        ColumnTree::Struct(subtrees) => {
            let mut objects = Vec::new();

            for (i, (field_name, subtree)) in subtrees.iter().enumerate() {
                if i == 0 {
                    // It's slightly annoying to get the number of elements before
                    // recursing or getting it from the caller
                    for subvalue in columntree_to_json_rows(subtree) {
                        let mut object = json::object::Object::with_capacity(subtrees.len());
                        object.insert(field_name, subvalue);
                        objects.push(object);
                    }
                } else {
                    for (subvalue, object) in iter::zip(
                        columntree_to_json_rows(subtree).into_iter(),
                        objects.iter_mut(),
                    ) {
                        object.insert(field_name, subvalue);
                    }
                }
            }
            objects.into_iter().map(JsonValue::Object).collect()
        }
        ColumnTree::List => Vec::new(), // TODO
        ColumnTree::Map => Vec::new(),  // TODO
        _ => todo!("{:?}", tree),
    }
}

#[test]
fn read_file() {
    let input_stream = reader::InputStream::from_local_file("orc/examples/TestOrcFile.test1.orc")
        .expect("Could not read .orc");
    let reader = reader::Reader::new(input_stream);

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