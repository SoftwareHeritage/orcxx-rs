// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

//! High-level parser for ORC files

use kind::Kind;
use reader::RowReader;
use vector;
use vector::ColumnVectorBatch;

/// Reads rows from ORC files to a tree of vectors (one for each column)
///
/// Wrapper for [`RowReader`] which provides an alternative to [`RowReader::row_batch`]
/// and [`RowReader::read_into`], by returning typed VectorBatches directly instead of
/// an [`vector::OwnedColumnVectorBatch`] which needs to be manually cast into
/// [`vector::StructVectorBatch`], [`vector::StringVectorBatch`], ...
pub struct StructuredRowReader<'a> {
    inner: &'a mut RowReader,
    vector_batch: vector::OwnedColumnVectorBatch,
}

impl<'a> StructuredRowReader<'a> {
    /// Consumes a [`RowReader`] to return a [`StructuredRowReader`]
    ///
    /// ``size`` is the number of rows to read at once.
    pub fn new(row_reader: &'a mut RowReader, size: u64) -> StructuredRowReader<'a> {
        StructuredRowReader {
            vector_batch: row_reader.row_batch(size),
            inner: row_reader,
        }
    }

    /// Returns the next batch of columns, if any.
    ///
    /// This slightly differs from [`Iterator::next`] as only one value can exist
    /// at any time (because they reuse the same data buffer).
    pub fn next<'b>(&'b mut self) -> Option<ColumnTree<'b>>
    where
        'a: 'b,
    {
        if !self.inner.read_into(&mut self.vector_batch) {
            // No more batches.
            return None;
        }

        Some(columnvectorbatch_to_columntree(
            self.vector_batch.borrow(),
            &self.inner.selected_kind(),
        ))
    }
}

/// A set of columns from ORC file
///
/// It is structured so that it follows the [`Kind`] selected in the
/// [`RowReader`]'s options (or the ORC file, by default)
#[derive(Debug)]
pub enum ColumnTree<'a> {
    Boolean(vector::LongVectorBatch<'a>),
    Byte(vector::LongVectorBatch<'a>),
    Short(vector::LongVectorBatch<'a>),
    Int(vector::LongVectorBatch<'a>),
    Long(vector::LongVectorBatch<'a>),
    Float(vector::DoubleVectorBatch<'a>),
    Double(vector::DoubleVectorBatch<'a>),
    String(vector::StringVectorBatch<'a>),
    Binary(vector::StringVectorBatch<'a>),
    Timestamp(vector::TimestampVectorBatch<'a>),
    /// Number of days since 1970-01-01
    Date(vector::LongVectorBatch<'a>),
    /// A column of lists
    ///
    /// The offsets are such that the first list is elements `offsets[0]` (inclusive) to
    /// `offsets[1]` (exclusive), the second list is elements `offsets[1]` (inclusive)
    /// to `offsets[2]` (exclusive), etc. and the last list is elements
    /// `offsets[offsets.len()-1]` to the end.
    ///
    /// None values in `offsets` indicates a null instead of a list.
    ///
    /// Therefore, offsets.collect().len() is exactly the number of lists.
    List {
        offsets: vector::LongVectorBatchIterator<'a>,
        elements: Box<ColumnTree<'a>>,
    },
    /// A column of maps
    ///
    /// The offsets are such that the first list is entries `offsets[0]` (inclusive) to
    /// `offsets[1]` (exclusive), the second list is entries `offsets[1]` (inclusive)
    /// to `offsets[2]` (exclusive), etc. and the last list is entries
    /// `offsets[offsets.len()-1]` to the end
    ///
    /// Therefore, offsets.len() is exactly the number of maps.
    Map {
        offsets: vector::LongVectorBatchIterator<'a>,
        keys: Box<ColumnTree<'a>>,
        elements: Box<ColumnTree<'a>>,
    },
    /// Pairs of (field_name, column_tree)
    ///
    /// if not [`None`], `not_null` is an array of booleans indicating which rows
    /// are present, so there are exactly as many values in the child `ColumnTree`s
    /// as there are true values in `not_null`.
    Struct {
        not_null: Option<&'a [i8]>,
        num_elements: u64, // TODO: deduplicate this with the not_null slice size?
        elements: Vec<(String, ColumnTree<'a>)>,
    },
    Decimal64(vector::Decimal64VectorBatch<'a>),
    Decimal128(vector::Decimal128VectorBatch<'a>),
    TimestampInstant, // TODO
}

fn columnvectorbatch_to_columntree<'a>(
    vector_batch: vector::BorrowedColumnVectorBatch<'a>,
    kind: &Kind,
) -> ColumnTree<'a> {
    match kind {
        Kind::Boolean => ColumnTree::Boolean(
            vector_batch
                .try_into_longs()
                .expect("Failed to cast booleans vector batch"),
        ),
        Kind::Byte => ColumnTree::Byte(
            vector_batch
                .try_into_longs()
                .expect("Failed to cast bytes vector batch"),
        ),
        Kind::Short => ColumnTree::Short(
            vector_batch
                .try_into_longs()
                .expect("Failed to cast shorts vector batch"),
        ),
        Kind::Int => ColumnTree::Int(
            vector_batch
                .try_into_longs()
                .expect("Failed to cast ints vector batch"),
        ),
        Kind::Long => ColumnTree::Long(
            vector_batch
                .try_into_longs()
                .expect("Failed to cast longs vector batch"),
        ),
        Kind::Float => ColumnTree::Float(
            vector_batch
                .try_into_doubles()
                .expect("Failed to cast floats vector batch"),
        ),
        Kind::Double => ColumnTree::Double(
            vector_batch
                .try_into_doubles()
                .expect("Failed to cast doubles vector batch"),
        ),
        Kind::String | Kind::Varchar(_) | Kind::Char(_) => ColumnTree::String(
            vector_batch
                .try_into_strings()
                .expect("Failed to cast strings vector batch"),
        ),
        Kind::Binary => ColumnTree::Binary(
            vector_batch
                .try_into_strings()
                .expect("Failed to cast strings vector batch"),
        ),
        Kind::Timestamp => ColumnTree::Timestamp(
            vector_batch
                .try_into_timestamps()
                .expect("Failed to cast timestamps vector batch"),
        ),
        Kind::Date => ColumnTree::Date(
            vector_batch
                .try_into_longs()
                .expect("Failed to cast date vector batch"),
        ),

        Kind::List(subtype) => {
            let lists_vector_batch = vector_batch
                .try_into_lists()
                .expect("Failed to cast lists vector_batch");
            ColumnTree::List {
                offsets: lists_vector_batch.iter_offsets(),
                elements: Box::new(columnvectorbatch_to_columntree(
                    lists_vector_batch.elements(),
                    subtype,
                )),
            }
        }
        Kind::Map { key, value } => {
            let maps_vector_batch = vector_batch
                .try_into_maps()
                .expect("Failed to cast maps vector_batch");
            ColumnTree::Map {
                offsets: maps_vector_batch.iter_offsets(),
                keys: Box::new(columnvectorbatch_to_columntree(
                    maps_vector_batch.keys(),
                    key,
                )),
                elements: Box::new(columnvectorbatch_to_columntree(
                    maps_vector_batch.elements(),
                    value,
                )),
            }
        }
        Kind::Struct(subtypes) => {
            let num_elements = vector_batch.num_elements();
            let not_null = vector_batch.not_null();
            if let Some(not_null) = not_null {
                assert_eq!(num_elements, not_null.len() as u64);
            }
            let elements = vector_batch
                .try_into_structs()
                .expect("Failed to cast structs vector_batch")
                .fields()
                .into_iter()
                .zip(subtypes.iter())
                .map(|(column, (name, kind))| {
                    (name.clone(), columnvectorbatch_to_columntree(column, kind))
                })
                .collect();
            ColumnTree::Struct {
                not_null,
                num_elements,
                elements,
            }
        }
        Kind::Union(_) => todo!("Union types"),
        Kind::Decimal { .. } => match vector_batch.try_into_decimals64() {
            Ok(vector_batch) => ColumnTree::Decimal64(vector_batch),
            Err(_) => ColumnTree::Decimal128(
                vector_batch
                    .try_into_decimals128()
                    .expect("Failed to cast decimal vector_batch"),
            ),
        },
        Kind::TimestampInstant => ColumnTree::TimestampInstant, // TODO
    }
}
