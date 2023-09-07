// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

//! Rust wrapper for the Apache ORC C++ library.
//!
//! Currently, it only allows reading files, not writing.
//!
//! ORC, short for Optimized Row Columnar, is a column-oriented data storage format.
//! As such, most of the APIs in this library operate on columns, rather than rows.
//! In order to work on rows, readers need to "zip" columns together.
//!
//! # Usage principles
//!
//! [`reader`] contains the entry points to parse a file, and reads into a
//! [`OwnedColumnVectorBatch`](vector::OwnedColumnVectorBatch) structure, which can be
//! `.borrow()`ed to get a [`BorrowedColumnVectorBatch`](vector::BorrowedColumnVectorBatch),
//! which implements most of the operations.
//!
//! This structure is untyped, and needs to be cast into the correct type, by calling
//! [`try_into_longs()`](vector::BorrowedColumnVectorBatch::try_into_longs),
//! [`try_into_strings()`](vector::BorrowedColumnVectorBatch::try_into_strings),
//! [`try_into_structs()`](vector::BorrowedColumnVectorBatch::try_into_structs), etc.
//!
//! While this works when parsing files whose structure is known, this is not very
//! practical. The [`StructuredRowReader`](structured_reader::StructuredRowReader) offers
//! an abstraction over [`RowReader`](reader::RowReader), which reads the schema of the
//! file (through [`selected_kind()`](reader::RowReader::selected_kind)) and dynamically
//! casts the vectors into the right type, recursively, in a
//! [`ColumnTree`](structured_reader::ColumnTree).
//!
//! For row-oriented access, see the [`orcxx_derive`](https://docs.rs/orcxx_derive) crate, which allows
//! `#[derive(OrcDeserialize)]` on structures in order to deserialize ORC files into
//! a structure instance for each row.
//! These structures can be deserialized either directly into vector batches with
//! [`deserialize::OrcDeserialize::read_from_vector_batch`], or iterated through
//! [`row_iterator::RowIterator`].
//!
//! # Panics
//!
//! May panic when requesting vector batches larger than `isize`;
//! this includes vector batches for variable-sized columns (maps and lists).
//! This is unlikely to happen on 64-bits machines (they would OOM first).
//!
//! [`row_iterator::RowIterator`] panics when underlying calls to
//! [`deserialize::OrcDeserialize::read_from_vector_batch`] error (so you may want to
//! avoid the former when working with non-trusted data).
//!
//! Panics may happen when the C++ library doesn't behave as expected, too.
//! C++ exceptions should be converted to Rust [`Result`]s, though.
//!
//! # Examples
//!
//! See the [`orcxx_derive` documentation](https://docs.rs/orcxx_derive/) for more high-level
//! examples and documentation.
//!
//! ```
//! use orcxx::reader;
//! use orcxx::vector::ColumnVectorBatch;
//!
//! let input_stream = reader::InputStream::from_local_file("orc/examples/TestOrcFile.test1.orc")
//!     .expect("Could not open");
//!
//! let reader = reader::Reader::new(input_stream).expect("Could not read");
//!
//! println!("{:#?}", reader.kind()); // Prints the type of columns in the file
//!
//! let mut row_reader = reader.row_reader(&reader::RowReaderOptions::default()).unwrap();
//! let mut batch = row_reader.row_batch(1024);
//!
//! let mut total_elements = 0;
//! let mut all_strings: Vec<String> = Vec::new();
//! while row_reader.read_into(&mut batch) {
//!     total_elements += (&batch).num_elements();
//!
//!     let struct_vector = batch.borrow().try_into_structs().unwrap();
//!     let vectors = struct_vector.fields();
//!
//!     for vector in vectors {
//!         match vector.try_into_strings() {
//!             Ok(string_vector) => {
//!                 for s in string_vector.iter() {
//!                     all_strings.push(
//!                         std::str::from_utf8(s.unwrap_or(b"<null>"))
//!                         .unwrap().to_owned())
//!                 }
//!             }
//!             Err(e) => {}
//!         }
//!     }
//! }
//!
//! assert_eq!(total_elements, 2);
//! assert_eq!(
//!     all_strings,
//!     vec!["\0\u{1}\u{2}\u{3}\u{4}", "", "hi", "bye"]
//!         .iter()
//!         .map(|s| s.to_owned())
//!         .collect::<Vec<_>>()
//! );
//! ```

extern crate cxx;
#[cfg(feature = "rayon")]
extern crate rayon;
extern crate unsafe_unwrap;

pub mod deserialize;
pub mod errors;
mod int128;
pub mod kind;
mod memorypool;
#[cfg(feature = "rayon")]
pub mod parallel_row_iterator;
pub mod reader;
pub mod row_iterator;
pub mod structured_reader;
pub mod vector;

#[cfg(feature = "json")]
extern crate chrono;
#[cfg(feature = "json")]
extern crate json;
extern crate rust_decimal;
#[cfg(feature = "json")]
pub mod to_json;
