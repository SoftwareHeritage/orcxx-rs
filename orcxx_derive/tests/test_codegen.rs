// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

extern crate orcxx;
extern crate orcxx_derive;

use orcxx::deserialize::{CheckableKind, OrcStruct};
use orcxx::kind::Kind;
use orcxx_derive::OrcDeserialize;

#[test]
fn test_basic() {
    #[derive(OrcDeserialize, Clone, Default, Debug, PartialEq)]
    struct Test {
        abc: String,
        def: i64,
    }

    Test::check_kind(&Kind::Struct(vec![
        ("abc".to_string(), Kind::String),
        ("def".to_string(), Kind::Long),
    ]))
    .unwrap();

    assert_eq!(Test::columns(), vec!["abc", "def"]);
}

#[test]
fn test_raw_literal() {
    #[derive(OrcDeserialize, Clone, Default, Debug, PartialEq)]
    struct Test {
        r#type: String,
    }

    Test::check_kind(&Kind::Struct(vec![("type".to_string(), Kind::String)])).unwrap();

    assert_eq!(Test::columns(), vec!["type"]);
}

#[test]
fn test_nested() {
    #[derive(OrcDeserialize, Clone, Default, Debug, PartialEq)]
    struct Test {
        abc: String,
        def: Inner,
        def2: Vec<Inner>,
    }

    #[derive(OrcDeserialize, Clone, Default, Debug, PartialEq)]
    struct Inner {
        ghi: i64,
        jkl: Vec<i32>,
    }

    let inner_kind = Kind::Struct(vec![
        ("ghi".to_string(), Kind::Long),
        ("jkl".to_string(), Kind::List(Box::new(Kind::Int))),
    ]);
    Test::check_kind(&Kind::Struct(vec![
        ("abc".to_string(), Kind::String),
        ("def".to_string(), inner_kind.clone()),
        ("def2".to_string(), Kind::List(Box::new(inner_kind))),
    ]))
    .unwrap();

    assert_eq!(
        Test::columns(),
        vec!["abc", "def.ghi", "def.jkl", "def2.ghi", "def2.jkl"]
    );
}
