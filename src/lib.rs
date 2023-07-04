// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

//! Rust wrapper for the Apache ORC C++ library.
//!
//! Currently, it only allows reading files, not writing.
//!
//! # Panics
//!
//! Never, assuming the underlying C++ library behaves as expected.
//!
//! C++ exceptions should be converted to Rust [Result].
//!
//! # Examples
//!
//! ```
//! use orcxx::reader;
//! use orcxx::vector::ColumnVectorBatch;
//!
//! let input_stream = reader::InputStream::from_local_file("orc/examples/TestOrcFile.test1.orc")
//!     .expect("Could not read");
//!
//! let reader = reader::Reader::new(input_stream);
//!
//! println!("{:#?}", reader.kind()); // Prints the type of columns in the file
//!
//! let mut row_reader = reader.row_reader(reader::RowReaderOptions::default());
//! let mut batch = row_reader.row_batch(1024);
//!
//! let mut total_elements = 0;
//! let mut all_strings: Vec<String> = Vec::new();
//! while row_reader.read_into(&mut batch) {
//!     total_elements += batch.num_elements();
//!
//!     let struct_vector = batch.borrow().try_into_structs().unwrap();
//!     let vectors = struct_vector.fields();
//!
//!     for vector in vectors {
//!         match vector.try_into_strings() {
//!             Ok(string_vector) => {
//!                 for s in string_vector.iter() {
//!                     all_strings.push(std::str::from_utf8(s).unwrap().to_owned())
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

pub mod kind;
mod memorypool;
pub mod reader;
pub mod structured_reader;
pub mod utils;
pub mod vector;

#[cfg(feature = "json")]
extern crate chrono;
#[cfg(feature = "json")]
extern crate json;
#[cfg(feature = "json")]
pub mod to_json;
