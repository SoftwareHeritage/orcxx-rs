# orcxx-rs

Rust wrapper for the official C++ library for Apache ORC.

It uses a submodule pointing to an Apache ORC release, builds its C++ part
(including vendored protobuf, lz4, zstd, ...), and links against that,
unless the `ORC_USE_SYSTEM_LIBRARIES` environment variable is set.
If it is, you need to make sure the dependencies are installed
(`apt-get install libprotoc-dev liblz4-dev libsnappy-dev libzstd-dev zlib1g-dev`
on Debian-based distributions).

If you have issues when building the crate with linker errors agains libhdfs,
you may try to define the `ORC_DISABLE_HDFS` environment variable.

The `orcxx_derive` crate provides a custom `derive` macro.

# `orcxx_derive` examples

## `RowIterator` API

<!-- Keep this in sync with orcxx_derive/src/lib.rs -->

```rust
extern crate orcxx;
extern crate orcxx_derive;

use std::num::NonZeroU64;

use orcxx::deserialize::{OrcDeserialize, OrcStruct};
use orcxx::row_iterator::RowIterator;
use orcxx::reader;
use orcxx_derive::OrcDeserialize;

// Define structure
#[derive(OrcDeserialize, Clone, Default, Debug, PartialEq, Eq)]
struct Test1 {
    long1: Option<i64>,
}

// Open file
let orc_path = "../orcxx/orc/examples/TestOrcFile.test1.orc";
let input_stream = reader::InputStream::from_local_file(orc_path).expect("Could not open .orc");
let reader = reader::Reader::new(input_stream).expect("Could not read .orc");

let batch_size = NonZeroU64::new(1024).unwrap();
let mut rows: Vec<Option<Test1>> = RowIterator::new(&reader, batch_size)
    .expect("Could not open ORC file")
    .collect();

assert_eq!(
    rows,
    vec![
        Some(Test1 {
            long1: Some(9223372036854775807)
        }),
        Some(Test1 {
            long1: Some(9223372036854775807)
        })
    ]
);
```

## Loop API

`RowIterator` clones structures before yielding them. This can be avoided by looping
and writing directly to a buffer:

<!-- Keep this in sync with orcxx_derive/src/lib.rs -->

```rust
extern crate orcxx;
extern crate orcxx_derive;

use orcxx::deserialize::{CheckableKind, OrcDeserialize, OrcStruct};
use orcxx::reader;
use orcxx_derive::OrcDeserialize;

// Define structure
#[derive(OrcDeserialize, Default, Debug, PartialEq, Eq)]
struct Test1 {
    long1: Option<i64>,
}

// Open file
let orc_path = "../orcxx/orc/examples/TestOrcFile.test1.orc";
let input_stream = reader::InputStream::from_local_file(orc_path).expect("Could not open .orc");
let reader = reader::Reader::new(input_stream).expect("Could not read .orc");

// Only read columns we need
let options = reader::RowReaderOptions::default().include_names(Test1::columns());

let mut row_reader = reader.row_reader(&options).expect("Could not open ORC file");
Test1::check_kind(&row_reader.selected_kind()).expect("Unexpected schema");

let mut rows: Vec<Option<Test1>> = Vec::new();

// Allocate work buffer
let mut batch = row_reader.row_batch(1024);

// Read structs until the end
while row_reader.read_into(&mut batch) {
    let new_rows = Option::<Test1>::from_vector_batch(&batch.borrow()).unwrap();
    rows.extend(new_rows);
}

assert_eq!(
    rows,
    vec![
        Some(Test1 {
            long1: Some(9223372036854775807)
        }),
        Some(Test1 {
            long1: Some(9223372036854775807)
        })
    ]
);
```

## Nested structures

The above two examples also work with nested structures:

```rust
extern crate orcxx;
extern crate orcxx_derive;

use orcxx_derive::OrcDeserialize;

#[derive(OrcDeserialize, Default, Debug, PartialEq)]
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

#[derive(OrcDeserialize, Default, Debug, PartialEq)]
struct Test1ItemOption {
    int1: Option<i32>,
    string1: Option<String>,
}
```

# `orcxx` examples

## ColumnTree API

Columns can also be read directly without writing their values to structures.
This is particularly useful to read files whose schema is not known at compile time.

## Low-level API

This reads batches directly from the C++ library, and leaves the Rust code to dynamically
cast base vectors to more specific types; here string vectors.

```rust
extern crate orcxx;
extern crate orcxx_derive;

use orcxx::reader;
use orcxx::vector::ColumnVectorBatch;

let input_stream = reader::InputStream::from_local_file("../orcxx/orc/examples/TestOrcFile.test1.orc")
    .expect("Could not open");

let reader = reader::Reader::new(input_stream).expect("Could not read");

println!("{:#?}", reader.kind()); // Prints the type of columns in the file

let mut row_reader = reader.row_reader(&reader::RowReaderOptions::default()).unwrap();
let mut batch = row_reader.row_batch(1024);

let mut total_elements = 0;
let mut all_strings: Vec<String> = Vec::new();
while row_reader.read_into(&mut batch) {
    total_elements += (&batch).num_elements();

    let struct_vector = batch.borrow().try_into_structs().unwrap();
    let vectors = struct_vector.fields();

    for vector in vectors {
        match vector.try_into_strings() {
            Ok(string_vector) => {
                for s in string_vector.iter() {
                    all_strings.push(
                        std::str::from_utf8(s.unwrap_or(b"<null>"))
                        .unwrap().to_owned())
                }
            }
            Err(e) => {}
        }
    }
}

assert_eq!(total_elements, 2);
assert_eq!(
    all_strings,
    vec!["\0\u{1}\u{2}\u{3}\u{4}", "", "hi", "bye"]
        .iter()
        .map(|s| s.to_owned())
        .collect::<Vec<_>>()
);
```
