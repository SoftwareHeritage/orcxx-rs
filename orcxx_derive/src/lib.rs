// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

//! Custom `derive` for the `orcxx` crate, to deserialize `structs` using Apache ORC C++ library.
//!
//! # Supported types
//!
//! Structures can have fields of the following types:
//!
//! * [`bool`], [`i8`], [`i16`], [`i32`], [`i64`], [`f32`], [`f64`], [`String`], [`Vec<u8>`](Vec),
//!   mapping to their respective ORC type
//! * `Vec<T>` when `T` is a supported type, mapping to an ORC list
//! * `HashMap<K, V>` and `Vec<(K, V)>` are not supported yet to deserialize ORC list
//!   (see <https://gitlab.softwareheritage.org/swh/devel/orcxx-rs/-/issues/1>)
//!
//! # About null values
//!
//! In order to support all ORC files, every single type should be wrapped in `Option`
//! (eg. `struct<a:int, b:list<string>>` in ORC should be
//! `a: Option<i32>, b: Option<Vec<Option<String>>>`), but this is cumbersome, and
//! may have high overhead if you need to check it.
//!
//! If you omit `Option`, then `orcxx_derive` will return an error early for files
//! containing null values, and avoid this overhead for files which don't.
//!
//! # Panics
//!
//! See `orcxx`'s documentation.
//!
//! # Example
//!
//! ```
//! extern crate orcxx;
//! extern crate orcxx_derive;
//!
//! use orcxx::deserialize::{OrcDeserialize, OrcStruct};
//! use orcxx::reader;
//! use orcxx_derive::OrcDeserialize;
//!
//! // Define structure
//! #[derive(OrcDeserialize, Default, Debug, PartialEq, Eq)]
//! struct Test1 {
//!     long1: Option<i64>,
//! }
//!
//! // Open file
//! let orc_path = "../orcxx/orc/examples/TestOrcFile.test1.orc";
//! let input_stream = reader::InputStream::from_local_file(orc_path).expect("Could not open .orc");
//! let reader = reader::Reader::new(input_stream).expect("Could not read .orc");
//!
//! // Only read columns we need
//! let options = reader::RowReaderOptions::default().include_names(Test1::columns());
//!
//! let mut row_reader = reader.row_reader(options).expect("'long1' is missing from the .orc");
//!
//! let mut rows: Vec<Option<Test1>> = Vec::new();
//!
//! // Allocate work buffer
//! let mut batch = row_reader.row_batch(1024);
//!
//! // Read structs until the end
//! while row_reader.read_into(&mut batch) {
//!     let new_rows = Option::<Test1>::from_vector_batch(&batch.borrow()).unwrap();
//!     rows.extend(new_rows);
//! }
//!
//! assert_eq!(
//!     rows,
//!     vec![
//!         Some(Test1 {
//!             long1: Some(9223372036854775807)
//!         }),
//!         Some(Test1 {
//!             long1: Some(9223372036854775807)
//!         })
//!     ]
//! );
//! ```
//!
//! It is also possible to nest structures:
//!
//! ```
//! extern crate orcxx;
//! extern crate orcxx_derive;
//!
//! use orcxx_derive::OrcDeserialize;
//!
//! #[derive(OrcDeserialize, Default, Debug, PartialEq)]
//! struct Test1Option {
//!     boolean1: Option<bool>,
//!     byte1: Option<i8>,
//!     short1: Option<i16>,
//!     int1: Option<i32>,
//!     long1: Option<i64>,
//!     float1: Option<f32>,
//!     double1: Option<f64>,
//!     bytes1: Option<Vec<u8>>,
//!     string1: Option<String>,
//!     list: Option<Vec<Option<Test1ItemOption>>>,
//! }
//!
//! #[derive(OrcDeserialize, Default, Debug, PartialEq)]
//! struct Test1ItemOption {
//!     int1: Option<i32>,
//!     string1: Option<String>,
//! }
//! ```

extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::*;

/// `#[derive(OrcDeserialize)] struct T { ... }` implements `OrcDeserialize for `T`,
/// `OrcDeserialize for `Option<T>`, and `CheckableKind for `T`,
#[proc_macro_derive(OrcDeserialize)]
pub fn orc_deserialize(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let tokens = match ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named, .. }),
            ..
        }) => impl_struct(
            &ast.ident,
            named
                .iter()
                .map(|field| {
                    field
                        .ident
                        .as_ref()
                        .expect("#ident must not have anonymous fields")
                })
                .collect(),
            named.iter().map(|field| &field.ty).collect(),
        ),
        Data::Struct(DataStruct { .. }) => panic!("#ident must have named fields"),
        _ => panic!("#ident must be a structure"),
    };

    //eprintln!("{}", tokens);

    tokens
}

fn impl_struct(ident: &Ident, field_names: Vec<&Ident>, field_types: Vec<&Type>) -> TokenStream {
    let num_fields = field_names.len();
    let unescaped_field_names: Vec<_> = field_names
        .iter()
        .map(|field_name| format_ident!("{}", field_name))
        .collect();

    let check_kind_impl = quote!(
        impl ::orcxx::deserialize::CheckableKind for #ident {
            fn check_kind(kind: &::orcxx::kind::Kind) -> Result<(), String> {
                use ::orcxx::kind::Kind;

                match kind {
                    Kind::Struct(fields) => {
                        let mut fields = fields.iter().enumerate();
                        let mut errors = Vec::new();
                        #(
                            match fields.next() {
                                Some((i, (field_name, field_type))) => {
                                    if field_name != stringify!(#unescaped_field_names) {
                                        errors.push(format!(
                                                "Field #{} must be called {}, not {}",
                                                i, stringify!(#unescaped_field_names), field_name))
                                    }
                                    else if let Err(s) = <#field_types>::check_kind(field_type) {
                                        errors.push(format!(
                                            "Field {} cannot be decoded: {}",
                                            stringify!(#unescaped_field_names), s));
                                    }
                                },
                                None => errors.push(format!(
                                    "Field {} is missing",
                                    stringify!(#unescaped_field_names)))
                            }
                        )*

                        if errors.is_empty() {
                            Ok(())
                        }
                        else {
                            Err(format!(
                                "{} cannot be decoded:\n\t{}",
                                stringify!(#ident),
                                errors.join("\n").replace("\n", "\n\t")))
                        }
                    }
                    _ => Err(format!(
                        "{} must be decoded from Kind::Struct, not {:?}",
                        stringify!(#ident),
                        kind))
                }
            }
        }
    );

    let orc_struct_impl = quote!(
        impl ::orcxx::deserialize::OrcStruct for #ident {
            fn columns_with_prefix(prefix: &str) -> Vec<String> {
                let mut columns = Vec::with_capacity(#num_fields);

                // Hack to get types. Hopefully the compiler notices we don't
                // actually use it at runtime.
                let instance: #ident = Default::default();

                #({
                    #[inline(always)]
                    fn add_columns<FieldType: ::orcxx::deserialize::OrcStruct>(columns: &mut Vec<String>, prefix: &str, _: FieldType) {
                        let mut field_name_prefix = prefix.to_string();
                        if prefix.len() != 0 {
                            field_name_prefix.push_str(".");
                        }
                        field_name_prefix.push_str(stringify!(#unescaped_field_names));
                        columns.extend(FieldType::columns_with_prefix(&field_name_prefix));
                    }
                    add_columns(&mut columns, prefix, instance.#field_names);
                })*
                columns
            }
        }
    );

    let prelude = quote!(
        use ::std::convert::TryInto;
        use ::std::collections::HashMap;

        use ::orcxx::deserialize::DeserializationError;
        use ::orcxx::deserialize::OrcDeserialize;
        use ::orcxx::vector::{ColumnVectorBatch, BorrowedColumnVectorBatch};
        use ::orcxx::deserialize::DeserializationTarget;

        let src = src.try_into_structs().map_err(DeserializationError::MismatchedColumnKind)?;
        let columns = src.fields();
        assert_eq!(
            columns.len(),
            #num_fields,
            "{} has {} fields, but got {} columns.",
            stringify!(ident), #num_fields, columns.len());
        let mut columns = columns.into_iter();

        let dst_len: u64 = dst.len().try_into().map_err(DeserializationError::UsizeOverflow)?;
        if src.num_elements() > dst_len {
            return Err(::orcxx::deserialize::DeserializationError::MismatchedLength { src: src.num_elements(), dst: dst_len });
        }
    );

    let read_from_vector_batch_impl = quote!(
        impl ::orcxx::deserialize::OrcDeserialize for #ident {
            fn read_from_vector_batch<'a, 'b, T> (
                src: &::orcxx::vector::BorrowedColumnVectorBatch, mut dst: &'b mut T
            ) -> Result<usize, ::orcxx::deserialize::DeserializationError>
            where
                &'b mut T: ::orcxx::deserialize::DeserializationTarget<'a, Item=#ident> + 'b {
                #prelude

                match src.not_null() {
                    None => {
                        for struct_ in dst.iter_mut() {
                            *struct_ = Default::default()
                        }
                    },
                    Some(not_null) => {
                        for (struct_, &b) in dst.iter_mut().zip(not_null) {
                            if b != 0 {
                                *struct_ = Default::default()
                            }
                        }
                    }
                }

                #(
                    let column: BorrowedColumnVectorBatch = columns.next().expect(
                        &format!("Failed to get '{}' column", stringify!(#field_names)));
                    OrcDeserialize::read_from_vector_batch::<orcxx::deserialize::MultiMap<&mut T, _>>(
                        &column,
                        &mut dst.map(|struct_| &mut struct_.#field_names),
                    )?;
                )*

                Ok(src.num_elements().try_into().unwrap())
            }
        }
    );

    let read_options_from_vector_batch_impl = quote!(
        impl ::orcxx::deserialize::OrcDeserializeOption for #ident {
            fn read_options_from_vector_batch<'a, 'b, T> (
                src: &::orcxx::vector::BorrowedColumnVectorBatch, mut dst: &'b mut T
            ) -> Result<usize, ::orcxx::deserialize::DeserializationError>
            where
                &'b mut T: ::orcxx::deserialize::DeserializationTarget<'a, Item=Option<#ident>> + 'b {
                #prelude

                match src.not_null() {
                    None => {
                        for struct_ in dst.iter_mut() {
                            *struct_ = Some(Default::default())
                        }
                    },
                    Some(not_null) => {
                        for (struct_, &b) in dst.iter_mut().zip(not_null) {
                            if b != 0 {
                                *struct_ = Some(Default::default())
                            }
                        }
                    }
                }

                #(
                    let column: BorrowedColumnVectorBatch = columns.next().expect(
                        &format!("Failed to get '{}' column", stringify!(#field_names)));
                    OrcDeserialize::read_from_vector_batch::<::orcxx::deserialize::MultiMap<&mut T, _>>(
                        &column,
                        &mut dst.map(|struct_| &mut unsafe { ::orcxx::deserialize::UnsafeUnwrap::unsafe_unwrap(struct_.as_mut()) }.#field_names),
                    )?;
                )*

                Ok(src.num_elements().try_into().unwrap())
            }
        }
    );

    quote!(
        #check_kind_impl
        #orc_struct_impl

        #read_from_vector_batch_impl
        #read_options_from_vector_batch_impl
    )
    .into()
}
