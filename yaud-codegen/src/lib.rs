/*
 *     Copyright (C) 2023  Fritz Ochsmann
 *
 *     This program is free software: you can redistribute it and/or modify
 *     it under the terms of the GNU Affero General Public License as published
 *     by the Free Software Foundation, either version 3 of the License, or
 *     (at your option) any later version.
 *
 *     This program is distributed in the hope that it will be useful,
 *     but WITHOUT ANY WARRANTY; without even the implied warranty of
 *     MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *     GNU Affero General Public License for more details.
 *
 *     You should have received a copy of the GNU Affero General Public License
 *     along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

#[macro_use]
extern crate quote;
#[macro_use]
extern crate darling;

use darling::FromDeriveInput;
use proc_macro::TokenStream;
use syn::{parse_macro_input, Data, DeriveInput, Type};

#[derive(FromDeriveInput)]
#[darling(attributes(writer))]
struct DataWriterOptions {
    name: Option<String>,
    table: String,
}

#[proc_macro_derive(DataWriter, attributes(writer))]
pub fn data_writer_macro_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let input_ident = &input.ident;
    let options: DataWriterOptions = FromDeriveInput::from_derive_input(&input).unwrap();
    let table = &options.table;

    let writer_ident = match options.name {
        Some(name) => format_ident!("{}", name),
        None => format_ident!("Write{}", input_ident),
    };

    // parse the struct data
    let fields = match input.data {
        Data::Struct(data) => data.fields,
        _ => panic!("Expected struct"),
    };
    let mut field_names = Vec::<syn::Ident>::new();
    let mut field_types = Vec::<Type>::new();
    fields.into_iter().for_each(|field| {
        field_names.push(field.ident.unwrap());
        field_types.push(field.ty);
    });

    let expanded = quote! {
        #[derive(Clone, Serialize, Getters, Setters)]
        #[set = "pub"]
        pub struct #writer_ident<'a> {
            #(
                #[serde(skip_serializing_if = "Option::is_none")]
                #field_names: Option<#field_types>,
            )*
            #[serde(skip)]
            connection: &'a crate::prelude::DatabaseConnection,
            #[serde(skip)]
            target: Option<&'a crate::Id>,
        }

        impl<'a> From<&'a crate::prelude::DatabaseConnection> for #writer_ident<'a> {
            fn from(connection: &'a crate::prelude::DatabaseConnection) -> Self {
                Self {
                    connection,
                    target: None,
                    #(
                        #field_names: None,
                    )*
                }
            }
        }

        impl<'a> std::future::IntoFuture for #writer_ident<'a> {
            type Output = Result<#input_ident>;
            type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + Sync + 'a>>;

            #[instrument(skip_all)]
            fn into_future(self) -> Self::IntoFuture {
                Box::pin(async move {
                    let result: #input_ident = if let Some(target) = self.target {
                        sql_span!(
                            self.connection
                                .update(target.to_thing())
                                .merge(self)
                                .await?
                        )
                    } else {
                        sql_span!(self.connection.create(#table).content(self).await?)
                    };

                    Ok(result)
                })
            }
        }

    };

    expanded.into()
}
