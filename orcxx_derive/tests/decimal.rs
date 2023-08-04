// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

extern crate orcxx;
extern crate orcxx_derive;
extern crate rust_decimal;
extern crate rust_decimal_macros;

use std::convert::TryInto;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use orcxx::deserialize::{CheckableKind, OrcDeserialize};
use orcxx::reader;
use orcxx::row_iterator::RowIterator;
use orcxx_derive::OrcDeserialize;

#[derive(OrcDeserialize, Clone, Debug, PartialEq, Default)]
struct Root {
    _col0: Option<Decimal>,
}

fn row_reader() -> reader::RowReader {
    let orc_path = "../orc/examples/decimal.orc";
    let input_stream = reader::InputStream::from_local_file(orc_path).expect("Could not open .orc");
    let reader = reader::Reader::new(input_stream).expect("Could not read .orc");

    let options = reader::RowReaderOptions::default();
    reader.row_reader(options).unwrap()
}

#[test]
fn test_decimal() {
    let mut row_reader = row_reader();
    Root::check_kind(&row_reader.selected_kind()).unwrap();

    let mut rows: Vec<Root> = Vec::new();

    let mut batch = row_reader.row_batch(1024);
    while row_reader.read_into(&mut batch) {
        let new_rows = Root::from_vector_batch(&batch.borrow()).unwrap();
        rows.extend(new_rows);
    }

    assert_eq!(
        rows.first(),
        Some(&Root {
            _col0: Some(dec!(-1000.5000))
        })
    );
    assert_eq!(
        rows.last(),
        Some(&Root {
            _col0: Some(dec!(1999.20000))
        })
    );
    assert!(rows.contains(&Root { _col0: None }));
    assert!(rows.contains(&Root {
        _col0: Some(dec!(1739.17400))
    }));
    assert!(!rows.contains(&Root {
        _col0: Some(dec!(1739.174000000000000000001))
    }));
    assert!(!rows.contains(&Root {
        _col0: Some(dec!(1739.17401))
    }));
}
