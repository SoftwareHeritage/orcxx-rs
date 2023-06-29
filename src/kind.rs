// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

/// Contains structures to represent ORC types.
///
/// See https://orc.apache.org/docs/types.html for details.
///
/// This module and its structures are named "kind" instead of "type" in order to
/// avoid clashes with the Rust keyword.
use cxx::let_cxx_string;

use utils::OrcResult;

#[cxx::bridge]
pub(crate) mod ffi {

    #[namespace = "orc"]
    unsafe extern "C++" {
        include!("orc/Type.hh");
    }

    #[namespace = "orc"]
    extern "C++" {
        type TypeKind;
    }

    // TODO: use #![variants_from_header] when https://github.com/dtolnay/cxx/pull/847
    // is stabilised
    #[namespace = "orc"]
    #[repr(i32)]
    enum TypeKind {
        BOOLEAN,
        BYTE,
        SHORT,
        INT,
        LONG,
        FLOAT,
        DOUBLE,
        STRING,
        BINARY,
        TIMESTAMP,
        LIST,
        MAP,
        STRUCT,
        UNION,
        DECIMAL,
        DATE,
        VARCHAR,
        CHAR,
        TIMESTAMP_INSTANT,
    }

    #[namespace = "orc"]
    unsafe extern "C++" {
        type Type;

        fn getKind(&self) -> TypeKind;
        fn getSubtypeCount(&self) -> u64;
        fn getSubtype(&self, childId: u64) -> *const Type;
        fn getFieldName(&self, childId: u64) -> &CxxString;
        fn getMaximumLength(&self) -> u64;
        fn getPrecision(&self) -> u64;
        fn getScale(&self) -> u64;
    }

    #[namespace = "orcxx_rs"]
    unsafe extern "C++" {
        include!("cpp-utils.hh");

        fn buildTypeFromString(input: &CxxString) -> Result<UniquePtr<Type>>;

        #[rust_name = "Type_toString"]
        #[namespace = "orcxx_rs::utils"]
        fn toString(type_: &Type) -> UniquePtr<CxxString>;
    }
}

/// A field of an ORC struct
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Field {
    pub name: String,
    pub kind: Kind,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Kind {
    Boolean,
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
    String,
    Binary,
    Timestamp,
    List(Box<Kind>),
    Map {
        key: Box<Kind>,
        value: Box<Kind>,
    },
    Struct(Vec<Field>),
    Union(Vec<Kind>),
    /// Infinite-precision number.
    ///
    /// Actually limited to u32 length in ORCv1:
    /// https://orc.apache.org/specification/ORCv1/#type-information
    Decimal {
        precision: u64,
        scale: u64,
    },
    Date,
    /// Variable-length character string.
    ///
    /// Actually limited to u32 length in ORCv1:
    /// https://orc.apache.org/specification/ORCv1/#type-information
    Varchar(u64),
    /// Fixed-length character string.
    ///
    /// Actually limited to u32 length in ORCv1:
    /// https://orc.apache.org/specification/ORCv1/#type-information
    Char(u64),
    TimestampInstant,
}

impl Kind {
    pub fn new(type_string: &str) -> OrcResult<Kind> {
        let_cxx_string!(type_string_cxx = type_string);
        let orc_type = ffi::buildTypeFromString(&type_string_cxx)?;
        Ok(Kind::new_from_orc_type(&orc_type))
    }

    pub(crate) fn new_from_orc_type(orc_type: &ffi::Type) -> Kind {
        match orc_type.getKind() {
            ffi::TypeKind::BOOLEAN => Kind::Boolean,
            ffi::TypeKind::BYTE => Kind::Byte,
            ffi::TypeKind::SHORT => Kind::Short,
            ffi::TypeKind::INT => Kind::Int,
            ffi::TypeKind::LONG => Kind::Long,
            ffi::TypeKind::FLOAT => Kind::Float,
            ffi::TypeKind::DOUBLE => Kind::Double,
            ffi::TypeKind::STRING => Kind::String,
            ffi::TypeKind::BINARY => Kind::Binary,
            ffi::TypeKind::TIMESTAMP => Kind::Timestamp,
            ffi::TypeKind::LIST => {
                assert_eq!(
                    orc_type.getSubtypeCount(),
                    1,
                    "orc::Type {:?} is a list but does not have exactly one subtype",
                    ffi::Type_toString(orc_type)
                );
                let sub_type = orc_type.getSubtype(0);

                // Safe because we just checked there is one subtype
                let sub_type = unsafe { &*sub_type };

                Kind::List(Box::new(Kind::new_from_orc_type(sub_type)))
            }
            ffi::TypeKind::MAP => {
                assert_eq!(
                    orc_type.getSubtypeCount(),
                    2,
                    "orc::Type {:?} is a map but does not have exactly two subtypes",
                    ffi::Type_toString(orc_type)
                );
                let key_type = orc_type.getSubtype(0);
                let value_type = orc_type.getSubtype(1);

                // Safe because we just checked there are two subtypes
                let key_type = unsafe { &*key_type };
                let value_type = unsafe { &*value_type };

                Kind::Map {
                    key: Box::new(Kind::new_from_orc_type(key_type)),
                    value: Box::new(Kind::new_from_orc_type(value_type)),
                }
            }
            ffi::TypeKind::STRUCT => Kind::Struct(
                (0..orc_type.getSubtypeCount())
                    .map(|i| {
                        let field_name = orc_type.getFieldName(i);
                        let sub_type = orc_type.getSubtype(i);

                        // Safe because i < subtypeCount
                        let sub_type = unsafe { &*sub_type };

                        Field {
                            // FIXME: we should probably return an Error on non-UTF8
                            // instead of using to_string_lossy
                            name: field_name.to_string_lossy().to_string(),
                            kind: Kind::new_from_orc_type(sub_type),
                        }
                    })
                    .collect(),
            ),
            ffi::TypeKind::UNION => Kind::Union(
                (0..orc_type.getSubtypeCount())
                    .map(|i| {
                        let sub_type = orc_type.getSubtype(i);
                        let sub_type = unsafe { &*sub_type }; // Safe because i < subtypeCount
                        Kind::new_from_orc_type(sub_type)
                    })
                    .collect(),
            ),
            ffi::TypeKind::DECIMAL => Kind::Decimal {
                precision: orc_type.getPrecision(),
                scale: orc_type.getScale(),
            },
            ffi::TypeKind::DATE => Kind::Date,
            ffi::TypeKind::VARCHAR => Kind::Varchar(orc_type.getMaximumLength()),
            ffi::TypeKind::CHAR => Kind::Char(orc_type.getMaximumLength()),
            ffi::TypeKind::TIMESTAMP_INSTANT => Kind::TimestampInstant,
            ffi::TypeKind { repr } => panic!("Unexpected value for orc::TypeKind: {}", repr),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn kind_from_orc_type_error() {
        assert!(Kind::new("").is_err());
        assert!(Kind::new("notatype").is_err());
        assert!(Kind::new("not a type").is_err());
    }

    // In the order of https://orc.apache.org/docs/types.html
    #[test]
    fn integer_kind_from_orc_type() {
        assert_eq!(Kind::new("boolean"), Ok(Kind::Boolean));
        assert_eq!(Kind::new("tinyint"), Ok(Kind::Byte));
        assert_eq!(Kind::new("smallint"), Ok(Kind::Short));
        assert_eq!(Kind::new("int"), Ok(Kind::Int));
        assert_eq!(Kind::new("bigint"), Ok(Kind::Long));
    }

    #[test]
    fn floating_point_kind_from_orc_type() {
        assert_eq!(Kind::new("float"), Ok(Kind::Float));
        assert_eq!(Kind::new("double"), Ok(Kind::Double));
    }

    #[test]
    fn string_kind_from_orc_type() {
        assert_eq!(Kind::new("string"), Ok(Kind::String));
        assert_eq!(Kind::new("char(10)"), Ok(Kind::Char(10)));
        assert_eq!(Kind::new("char()"), Ok(Kind::Char(0)));
        assert_eq!(Kind::new("char(0)"), Ok(Kind::Char(0)));
        assert_eq!(Kind::new("char(276447232)"), Ok(Kind::Char(276447232)));
        assert_eq!(Kind::new("varchar(10)"), Ok(Kind::Varchar(10)));
        assert_eq!(Kind::new("varchar()"), Ok(Kind::Varchar(0)));
        assert_eq!(Kind::new("varchar(0)"), Ok(Kind::Varchar(0)));
        assert_eq!(
            Kind::new("varchar(276447232)"),
            Ok(Kind::Varchar(276447232))
        );

        assert!(Kind::new("char").is_err());
        assert!(Kind::new("char(").is_err());
        assert!(Kind::new("varchar").is_err());
        assert!(Kind::new("varchar(").is_err());
    }

    #[test]
    fn binary_kind_from_orc_type() {
        assert_eq!(Kind::new("binary"), Ok(Kind::Binary));
    }

    #[test]
    fn decimal_kind_from_orc_type() {
        assert_eq!(
            Kind::new("decimal(1, 1)"),
            Ok(Kind::Decimal {
                precision: 1,
                scale: 1
            })
        );
        assert_eq!(
            Kind::new("decimal(1000, 1)"),
            Ok(Kind::Decimal {
                precision: 1000,
                scale: 1
            })
        );
        assert_eq!(
            Kind::new("decimal(1, 1000)"),
            Ok(Kind::Decimal {
                precision: 1,
                scale: 1000
            })
        );
        assert_eq!(
            Kind::new("decimal(276447232, 276447232)"),
            Ok(Kind::Decimal {
                precision: 276447232,
                scale: 276447232
            })
        );
        assert!(Kind::new("decimal()").is_err());
    }

    #[test]
    fn datetime_kind_from_orc_type() {
        assert_eq!(Kind::new("timestamp"), Ok(Kind::Timestamp));
        assert_eq!(Kind::new("date"), Ok(Kind::Date));
        assert_eq!(
            Kind::new("timestamp with local time zone"),
            Ok(Kind::TimestampInstant)
        );
    }

    #[test]
    fn struct_kind_from_orc_type() {
        assert_eq!(Kind::new("struct<>"), Ok(Kind::Struct(vec![])));
        assert_eq!(
            Kind::new("struct<a:boolean>"),
            Ok(Kind::Struct(vec![Field {
                name: "a".to_owned(),
                kind: Kind::Boolean
            }]))
        );
        assert_eq!(
            Kind::new("struct<a:boolean,b:smallint,c:int,d:bigint>"),
            Ok(Kind::Struct(vec![
                Field {
                    name: "a".to_owned(),
                    kind: Kind::Boolean
                },
                Field {
                    name: "b".to_owned(),
                    kind: Kind::Short
                },
                Field {
                    name: "c".to_owned(),
                    kind: Kind::Int
                },
                Field {
                    name: "d".to_owned(),
                    kind: Kind::Long
                }
            ]))
        );
        assert_eq!(
            Kind::new("struct<a:boolean,b:struct<b1:smallint,b2:int>,c:bigint>"),
            Ok(Kind::Struct(vec![
                Field {
                    name: "a".to_owned(),
                    kind: Kind::Boolean
                },
                Field {
                    name: "b".to_owned(),
                    kind: Kind::Struct(vec![
                        Field {
                            name: "b1".to_owned(),
                            kind: Kind::Short
                        },
                        Field {
                            name: "b2".to_owned(),
                            kind: Kind::Int
                        }
                    ])
                },
                Field {
                    name: "c".to_owned(),
                    kind: Kind::Long
                }
            ]))
        );

        assert!(Kind::new("struct<boolean>").is_err());
    }

    #[test]
    fn list_kind_from_orc_type() {
        assert_eq!(
            Kind::new("array<boolean>"),
            Ok(Kind::List(Box::new(Kind::Boolean)))
        );
        assert_eq!(
            Kind::new("array<struct<a:boolean,b:smallint,c:int,d:bigint>>"),
            Ok(Kind::List(Box::new(Kind::Struct(vec![
                Field {
                    name: "a".to_owned(),
                    kind: Kind::Boolean
                },
                Field {
                    name: "b".to_owned(),
                    kind: Kind::Short
                },
                Field {
                    name: "c".to_owned(),
                    kind: Kind::Int
                },
                Field {
                    name: "d".to_owned(),
                    kind: Kind::Long
                }
            ]))))
        );

        assert!(Kind::new("array<>").is_err());
        assert!(Kind::new("array<a:boolean>").is_err());
    }

    #[test]
    fn map_kind_from_orc_type() {
        assert_eq!(
            Kind::new("map<string,boolean>"),
            Ok(Kind::Map {
                key: Box::new(Kind::String),
                value: Box::new(Kind::Boolean)
            })
        );

        assert!(Kind::new("map<>").is_err());
        assert!(Kind::new("map<boolean>").is_err());
        assert!(Kind::new("map<a:boolean>").is_err());
    }

    #[test]
    fn union_kind_from_orc_type() {
        assert_eq!(Kind::new("uniontype<>"), Ok(Kind::Union(vec![])));
        assert_eq!(
            Kind::new("uniontype<string>"),
            Ok(Kind::Union(vec![Kind::String]))
        );
        assert_eq!(
            Kind::new("uniontype<string,boolean>"),
            Ok(Kind::Union(vec![Kind::String, Kind::Boolean]))
        );

        assert!(Kind::new("uniontype<a:boolean>").is_err());
    }
}
