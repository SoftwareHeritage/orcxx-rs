// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

extern crate orcxx;
extern crate orcxx_derive;
extern crate rust_decimal;
extern crate rust_decimal_macros;

use orcxx::deserialize::{CheckableKind, OrcDeserialize};
use orcxx::reader;
use orcxx::Timestamp;

fn row_reader() -> reader::RowReader {
    let orc_path = "../orcxx/orc/examples/TestOrcFile.testTimestamp.orc";
    let input_stream = reader::InputStream::from_local_file(orc_path).expect("Could not open .orc");
    let reader = reader::Reader::new(input_stream).expect("Could not read .orc");

    let options = reader::RowReaderOptions::default();
    reader.row_reader(&options).unwrap()
}

#[test]
fn test_timestamp() {
    let mut row_reader = row_reader();
    Timestamp::check_kind(&row_reader.selected_kind()).unwrap();

    let mut rows: Vec<Timestamp> = Vec::new();

    let mut batch = row_reader.row_batch(1024);
    while row_reader.read_into(&mut batch) {
        let new_rows = Timestamp::from_vector_batch(&batch.borrow()).unwrap();
        rows.extend(new_rows);
    }

    assert_eq!(
        rows,
        vec![
            Timestamp {
                seconds: 2114380800,
                nanoseconds: 999000
            },
            Timestamp {
                seconds: 1041379200,
                nanoseconds: 222
            },
            Timestamp {
                seconds: 915148800,
                nanoseconds: 999999999
            },
            Timestamp {
                seconds: 788918400,
                nanoseconds: 688888888
            },
            Timestamp {
                seconds: 1009843200,
                nanoseconds: 100000000
            },
            Timestamp {
                seconds: 1267488000,
                nanoseconds: 9001
            },
            Timestamp {
                seconds: 1104537600,
                nanoseconds: 2229
            },
            Timestamp {
                seconds: 1136073600,
                nanoseconds: 900203003
            },
            Timestamp {
                seconds: 1041379200,
                nanoseconds: 800000007
            },
            Timestamp {
                seconds: 838944000,
                nanoseconds: 723100809
            },
            Timestamp {
                seconds: 909964800,
                nanoseconds: 857340643
            },
            Timestamp {
                seconds: 1222905600,
                nanoseconds: 0
            }
        ]
    );
}
