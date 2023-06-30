// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

//! High-level parser for ORC files

use kind::Kind;
use reader::RowReader;
use vector;

/// Reads rows from ORC files to a tree of vectors (one for each column)
///
/// Wrapper for [RowReader] which provides an alternative to [RowReader::row_batch]
/// and [RowReader::read_into], by returning typed VectorBatches directly instead of
/// an [vector::OwnedColumnVectorBatch] which needs to be manually cast into
/// [vector::StructVectorBatch], [vector::StringVectorBatch], ...
pub struct StructuredRowReader<'a> {
    inner: &'a mut RowReader,
    vector_batch: vector::OwnedColumnVectorBatch,
}

impl<'a> StructuredRowReader<'a> {
    /// Consumes a [RowReader] to return a [StructuredRowReader]
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
    /// This slightly differs from [Iterator::next] as only one value can exist
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
/// It is structured so that it follows the [kind::Kind] selected in the
/// [RowReader]'s options (or the ORC file, by default)
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
    Timestamp, // TODO
    List,      // TODO
    Map,       // TODO
    /// Pairs of (field_name, column_tree)
    Struct(Vec<(String, ColumnTree<'a>)>),
    Union,            // TODO
    Decimal,          // TODO
    Date,             // TODO
    Varchar,          // TODO
    Char,             // TODO
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
        Kind::String => ColumnTree::String(
            vector_batch
                .try_into_strings()
                .expect("Failed to cast strings vector batch"),
        ),
        Kind::Binary => ColumnTree::Binary(
            vector_batch
                .try_into_strings()
                .expect("Failed to cast strings vector batch"),
        ),
        Kind::Timestamp => ColumnTree::Timestamp,    // TODO
        Kind::List(subtype) => ColumnTree::List,     // TODO
        Kind::Map { key, value } => ColumnTree::Map, // TODO
        Kind::Struct(subtypes) => ColumnTree::Struct(
            vector_batch
                .try_into_structs()
                .expect("Failed to cast structs vector_batch")
                .fields()
                .into_iter()
                .zip(subtypes.iter())
                .map(|(column, (name, kind))| {
                    (name.clone(), columnvectorbatch_to_columntree(column, kind))
                })
                .collect(),
        ),
        Kind::Union(subtypes) => ColumnTree::Union, // TODO
        Kind::Decimal { precision, scale } => ColumnTree::Decimal, // TODO
        Kind::Date => ColumnTree::Date,             // TODO
        Kind::Varchar(_) => ColumnTree::Varchar,    // TODO
        Kind::Char(_) => ColumnTree::Char,          // TODO
        Kind::TimestampInstant => ColumnTree::TimestampInstant, // TODO
    }
}
