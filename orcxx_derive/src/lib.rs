// Copyright (C) 2023 The Software Heritage developers
// See the AUTHORS file at the top-level directory of this distribution
// License: GNU General Public License version 3, or any later version
// See top-level LICENSE file for more information

//! Custom `derive` for the `orcxx` crate, to deserialize `structs` using  Apache ORC C++ library.
//!
//! # Panics
//!
//! TODO
//!
//! # Example
//!
//! ```
//! extern crate orcxx;
//! extern crate orcxx_derive;
//!
//! use orcxx::deserialize::OrcDeserializable;
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
//! let orc_path = "../orc/examples/TestOrcFile.test1.orc";
//! let input_stream = reader::InputStream::from_local_file(orc_path).expect("Could not open .orc");
//! let reader = reader::Reader::new(input_stream).expect("Could not read .orc");
//!
//! // Setup reader (list of columns must match fields in Test1)
//! let options = reader::RowReaderOptions::default().include_names(["long1"]);
//! let mut row_reader = reader.row_reader(options).expect("'long1' is missing from the .orc");
//!
//! let mut rows: Vec<Option<Test1>> = Vec::new();
//!
//! // Allocate work buffer
//! let mut batch = row_reader.row_batch(1024);
//!
//! // Read structs until the end
//! while row_reader.read_into(&mut batch) {
//!     let new_rows = Test1::options_from_vector_batch(&batch.borrow()).unwrap();
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

extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;
extern crate unsafe_unwrap;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::*;

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

    let impl_ = quote!(
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
                                    if field_name != stringify!(#field_names) {
                                        errors.push(format!(
                                                "Field #{} must be called {}, not {}",
                                                i, stringify!(#field_names), field_name))
                                    }
                                    else if let Err(s) = <#field_types>::check_kind(field_type) {
                                        errors.push(format!(
                                            "Field {} cannot be decoded: {}",
                                            stringify!(#field_names), s));
                                    }
                                },
                                None => errors.push(format!(
                                    "Field {} is missing",
                                    stringify!(#field_names)))
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

        impl ::orcxx::deserialize::OrcDeserializable for #ident {
            fn read_options_from_vector_batch<'a, 'b, T> (
                src: &::orcxx::vector::BorrowedColumnVectorBatch, mut dst: &'b mut T
            ) -> Result<(), ::orcxx::deserialize::DeserializationError>
            where
                &'b mut T: ::orcxx::deserialize::DeserializationTarget<'a, Item=Option<#ident>> + 'b {
                extern crate unsafe_unwrap;

                use ::std::convert::TryInto;
                use ::std::collections::HashMap;

                use ::orcxx::deserialize::DeserializationError;
                use ::orcxx::deserialize::OrcDeserializable;
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
                assert_eq!(
                    src.num_elements(),
                    dst_len);

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
                    OrcDeserializable::read_options_from_vector_batch::<orcxx::deserialize::MultiMap<&mut T, _>>(
                        &column,
                        &mut dst.map(|struct_| &mut unsafe { unsafe_unwrap::UnsafeUnwrap::unsafe_unwrap(struct_.as_mut()) }.#field_names),
                    )?;
                )*

                Ok(())
            }
        }
    );

    impl_.into()
}
