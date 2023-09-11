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
extern crate darling;

use darling::{FromDeriveInput, FromField};
use proc_macro::TokenStream;
use syn::{parse_macro_input, Data, DeriveInput, Path, Type};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(writer))]
struct DataWriterOptions {
    name: Option<String>,
    table: String,
    #[darling(default)]
    impl_full_request: bool,
}

#[derive(Debug, FromField)]
#[darling(attributes(writer))]
struct DataWriterFieldOptions {
    #[darling(default)]
    skip: bool,
    #[darling(default)]
    skip_full: bool,
    full: Option<Path>,
    #[darling(default)]
    editable: bool,
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

    let mut full_field_names = Vec::<syn::Ident>::new();
    let mut full_field_types = Vec::<Type>::new();

    let mut full_field_default_names = Vec::<syn::Ident>::new();
    let mut full_field_default_path = Vec::<syn::Path>::new();

    let mut editable_field_names = Vec::<syn::Ident>::new();
    let mut editable_field_types = Vec::<Type>::new();
    let mut editable_field_setter = Vec::<syn::Ident>::new();

    fields.into_iter().for_each(|field| {
        let options = DataWriterFieldOptions::from_field(&field).unwrap();

        if !options.skip {
            field_names.push(field.clone().ident.unwrap());
            field_types.push(field.clone().ty);

            if !options.skip_full {
                if let Some(path) = options.full {
                    full_field_default_names.push(field.clone().ident.unwrap());
                    full_field_default_path.push(path);
                } else {
                    full_field_names.push(field.clone().ident.unwrap());
                    full_field_types.push(field.clone().ty);
                }
            }

            if options.editable {
                editable_field_setter.push(format_ident!("set_{}", field.ident.as_ref().unwrap()));
                editable_field_names.push(field.ident.unwrap());
                editable_field_types.push(field.ty);
            }
        }
    });

    let full_request = if options.impl_full_request && !full_field_names.is_empty() {
        let full_request = format_ident!("{}Request", &writer_ident);

        quote! {
            #[derive(Deserialize, Clone, Debug, JsonSchema)]
            pub struct #full_request {
                #(
                    pub #full_field_names: #full_field_types,
                )*
            }

            impl<'a> #writer_ident<'a> {
                pub fn with_request(&mut self, request: #full_request) -> &mut Self {
                    #(
                      self.#full_field_names = Some(request.#full_field_names);
                    )*

                    #(
                      self.#full_field_default_names = Some(#full_field_default_path);
                    )*

                    self
                }
            }
        }
    } else {
        quote! {}
    };

    let editable = if !editable_field_names.is_empty() {
        let editable_ident = format_ident!("Edit{}", input_ident);

        quote! {
            #[derive(Deserialize, Debug, Clone, JsonSchema)]
            pub struct #editable_ident {
                #(
                  #editable_field_names: Option<#editable_field_types>,
                )*
            }

            impl #editable_ident {
                pub fn to_writer(self, connection: &DatabaseConnection) -> #writer_ident {
                    #writer_ident::from(connection)
                        #(
                            .#editable_field_setter(self.#editable_field_names)
                        )*
                        .to_owned()
                }
            }
        }
    } else {
        quote! {}
    };

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
                                .unwrap()
                        )
                    } else {
                        sql_span!(self.connection.create(#table).content(self).await?.into_iter().next().unwrap())
                    };

                    Ok(result)
                })
            }
        }

        #full_request

        #editable
    };

    expanded.into()
}
