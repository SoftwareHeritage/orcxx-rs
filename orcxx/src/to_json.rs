// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

//! Produces line-separated JSON documents from ORC
//!
//! # Example
//!
//! ```
//! use orcxx::*;
//!
//! let orc_path = "../orc/examples/TestOrcFile.test1.orc";
//! let input_stream = reader::InputStream::from_local_file(orc_path).expect("Could not open .orc");
//! let reader = reader::Reader::new(input_stream).expect("Could not read .orc");
//!
//! let mut row_reader = reader.row_reader(reader::RowReaderOptions::default()).unwrap();
//!
//! let mut structured_row_reader = structured_reader::StructuredRowReader::new(&mut row_reader, 1024);
//!
//! while let Some(columns) = structured_row_reader.next() {
//!     for object in to_json::columntree_to_json_rows(columns) {
//!         println!("{}", json::stringify_pretty(object, 4));
//!     }
//! }
//! ```

use std::convert::TryInto;
use std::iter;

use json::JsonValue;
use rust_decimal::prelude::ToPrimitive;

use structured_reader::ColumnTree;
use vector::DecimalVectorBatch;

fn map_nullable_json_values<V, C: Iterator<Item = Option<V>>, F>(column: C, f: F) -> Vec<JsonValue>
where
    F: Fn(V) -> JsonValue,
{
    column
        .map(|v| match v {
            None => JsonValue::Null,
            Some(v) => f(v),
        })
        .collect()
}

/// Given a set of columns (as a [`ColumnTree`]), returns a vector of rows
/// represented as a JSON-like data structure.
pub fn columntree_to_json_rows<'a>(tree: ColumnTree<'a>) -> Vec<JsonValue> {
    match tree {
        ColumnTree::Boolean(column) => {
            map_nullable_json_values(column.iter(), |b| JsonValue::Boolean(b != 0))
        }
        ColumnTree::Byte(column)
        | ColumnTree::Short(column)
        | ColumnTree::Int(column)
        | ColumnTree::Long(column) => {
            map_nullable_json_values(column.iter(), |b| JsonValue::Number(b.into()))
        }
        ColumnTree::Float(column) | ColumnTree::Double(column) => {
            map_nullable_json_values(column.iter(), |b| JsonValue::Number(b.into()))
        }
        ColumnTree::String(column) => map_nullable_json_values(column.iter(), |s| {
            JsonValue::String(String::from_utf8_lossy(s).into_owned())
        }),
        ColumnTree::Timestamp(column) => {
            map_nullable_json_values(column.iter(), |(seconds, nanoseconds)| {
                let mut s = chrono::NaiveDateTime::from_timestamp_opt(
                    seconds,
                    nanoseconds
                        .try_into()
                        .expect("More than 2**32 nanoseconds in a second"),
                )
                .expect("Could not create NaiveDateTime")
                .format("%Y-%m-%d %H:%M:%S.%f")
                .to_string()
                .trim_end_matches("0")
                .to_string();
                if s.ends_with(".") {
                    s.push('0');
                }
                JsonValue::String(s)
            })
        }
        ColumnTree::Date(column) => map_nullable_json_values(column.iter(), |days| {
            let substract = days <= 0;
            let days = chrono::Days::new(
                days.abs()
                    .try_into()
                    .expect("Failed to convert positive days from i64 to u64"),
            );
            let date = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
            let date = if substract {
                date.checked_sub_days(days)
            } else {
                date.checked_add_days(days)
            };

            let s = date
                .expect("Overflowed NaiveDate")
                .format("%Y-%m-%d")
                .to_string();
            JsonValue::String(s)
        }),
        ColumnTree::Decimal64(column) => map_nullable_json_values(column.iter(), |n| {
            JsonValue::Number(
                n.to_f64()
                    .expect("Decimal cannot be represented with f64")
                    .into(),
            )
        }),
        ColumnTree::Decimal128(column) => map_nullable_json_values(column.iter(), |n| {
            JsonValue::Number(
                n.to_f64()
                    .expect("Decimal cannot be represented with f64")
                    .into(),
            )
        }),
        ColumnTree::Binary(column) => map_nullable_json_values(column.iter(), |s| {
            JsonValue::Array(
                s.into_iter()
                    .map(|&byte| JsonValue::Number(byte.into()))
                    .collect(),
            )
        }),
        ColumnTree::Struct {
            not_null,
            num_elements,
            elements,
        } => {
            if let Some(not_null) = not_null {
                assert_eq!(num_elements, not_null.len() as u64);
            }
            let num_fields = elements.len();
            let num_not_null_elements = match not_null {
                None => num_elements,
                Some(not_null) => not_null
                    .iter()
                    .filter(|&&b| b != 0)
                    .count()
                    .try_into()
                    .expect("Could not convert usize to u64"),
            };

            let mut objects: Vec<_> = (0..num_not_null_elements)
                .map(|_| json::object::Object::with_capacity(num_fields))
                .collect();

            for (field_name, subtree) in elements.into_iter() {
                for (subvalue, object) in iter::zip(
                    columntree_to_json_rows(subtree).into_iter(),
                    objects.iter_mut(),
                ) {
                    object.insert(&field_name, subvalue);
                }
            }

            match not_null {
                None => objects.into_iter().map(JsonValue::Object).collect(),
                Some(not_null) => {
                    let mut values = Vec::with_capacity(not_null.len());
                    let mut objects_iter = objects.into_iter();
                    for &b in not_null {
                        if b == 0 {
                            values.push(JsonValue::Null);
                        } else {
                            values.push(JsonValue::Object(
                                objects_iter
                                    .next()
                                    .expect("Struct field vector unexpectedly too short"),
                            ));
                        }
                    }

                    assert_eq!(
                        objects_iter.next(),
                        None,
                        "Struct field vector unexpectedly too long"
                    );
                    values
                }
            }
        }
        ColumnTree::List {
            mut offsets,
            elements,
        } => {
            let values = columntree_to_json_rows(*elements);
            let mut arrays: Vec<Option<Vec<_>>> = Vec::new(); // TODO: try to guess the capacity

            let mut next_split = None;
            loop {
                let offset = offsets.next();
                match offset {
                    // Vector only contains nulls (or is empty)
                    None => break,
                    // First values in the vector are nulls
                    Some(None) => arrays.push(None),
                    // First non-null value in the vector
                    Some(Some(first_split)) => {
                        next_split = Some(first_split as usize);
                        break;
                    }
                }
            }
            for (i, value) in values.into_iter().enumerate() {
                while Some(i) == next_split {
                    let offset = offsets.next();
                    match offset {
                        // Last list of vector
                        None => {
                            arrays.push(Some(Vec::new()));
                            next_split = None
                        }
                        // New null value
                        Some(None) => arrays.push(None),
                        // New list value
                        Some(Some(j)) => {
                            arrays.push(Some(Vec::new()));
                            next_split = Some(j as usize);
                        }
                    }
                }
                arrays.last_mut().unwrap().as_mut().unwrap().push(value);
            }

            // Fill nulls at the end
            while let Some(_) = next_split {
                arrays.push(None);
                next_split = offsets.next().unwrap_or(None).map(|offset| offset as usize);
            }

            arrays
                .into_iter()
                .map(|v| match v {
                    Some(vec) => JsonValue::Array(vec),
                    None => JsonValue::Null,
                })
                .collect()
        }
        ColumnTree::Map {
            mut offsets,
            keys,
            elements,
        } => {
            let keys = columntree_to_json_rows(*keys);
            let values = columntree_to_json_rows(*elements);
            let mut maps: Vec<_> = Vec::new(); // TODO: try to guess the capacity

            let mut next_split = None;
            loop {
                let offset = offsets.next();
                match offset {
                    // Vector only contains nulls (or is empty)
                    None => break,
                    // First values in the vector are nulls
                    Some(None) => maps.push(None),
                    // First non-null value in the vector
                    Some(Some(first_split)) => {
                        next_split = Some(first_split as usize);
                        break;
                    }
                }
            }
            for (i, (key, value)) in iter::zip(keys.into_iter(), values.into_iter()).enumerate() {
                while Some(i) == next_split {
                    let offset = offsets.next();
                    match offset {
                        // Last map of vector
                        None => {
                            maps.push(Some(Vec::new()));
                            next_split = None
                        }
                        // New null value
                        Some(None) => maps.push(None),
                        // New map value
                        Some(Some(j)) => {
                            maps.push(Some(Vec::new()));
                            next_split = Some(j as usize);
                        }
                    }
                }
                let mut object = json::object::Object::with_capacity(2);
                object.insert("key", key);
                object.insert("value", value);
                maps.last_mut()
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .push(JsonValue::Object(object));
            }

            // Fill nulls at the end
            while let Some(_) = next_split {
                maps.push(None);
                next_split = offsets.next().unwrap_or(None).map(|offset| offset as usize);
            }

            maps.into_iter()
                .map(|o| match o {
                    None => JsonValue::Null,
                    Some(o) => JsonValue::Array(o),
                })
                .collect()
        }
        _ => todo!("{:?}", tree),
    }
}
