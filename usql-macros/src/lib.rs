extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{Data, DataEnum, DataStruct, DeriveInput, Field, Generics, parse_macro_input};

#[proc_macro_derive(FromRow)]
pub fn derive_from_value(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident,
        data,
        generics,
        ..
    } = parse_macro_input!(input as DeriveInput);

    match data {
        Data::Enum(e) => derive_enum_from_value(ident, generics, e),
        Data::Struct(e) => derive_struct_from_value(ident, generics, e),
        _ => panic!(""),
    }
}

fn derive_struct_from_value(name: Ident, generics: Generics, item: DataStruct) -> TokenStream {
    let vaerdi_name = format_ident!("usql");

    let len = item.fields.len();

    if len == 1 {
        let field = item.fields.iter().next().unwrap();
        // NewType type struct
        if field.ident.is_none() {
            return derive_new_type_struct_from_value(name, generics, &item, field);
        }
    }

    let (ty, imp, wh) = generics.split_for_impl();

    let fields = item
        .fields
        .iter()
        .map(|m| {
            //
            let name = m.ident.as_ref().unwrap();
            let name_str = name.to_string();
            let ty = &m.ty;
            quote!(
                #name: row.try_get::<#ty, _>(#name_str)?
            )
        })
        .collect::<Vec<_>>();

    quote!(

        impl #imp #vaerdi_name::FromRow for #name #ty #wh {
            fn from_row<B: #vaerdi_name::core::Connector>(row: #vaerdi_name::Row<B>) -> Result<#name #ty, #vaerdi_name::Error<B>> {
               
                Ok(#name {
                    #(#fields),*
                })

            }
        }

    )
    .into()
}

fn derive_new_type_struct_from_value(
    name: Ident,
    generics: Generics,
    _item: &DataStruct,
    field: &Field,
) -> TokenStream {
    let vaerdi_name = format_ident!("vaerdi");
    let ty = &field.ty;
    let (gty, imp, wh) = generics.split_for_impl();

    quote!(

        impl #imp #vaerdi_name::convert::FromValue for #name #gty #wh {
            type Error = #vaerdi_name::ConvertError;
            fn from_value(from: #vaerdi_name::Value) -> Result<#name, Self::Error> {
                Ok(#name(<#ty as #vaerdi_name::convert::FromValue>::from_value(from)?))
            }
        }



    )
    .into()
}

fn derive_enum_from_value(enum_name: Ident, generics: Generics, item: DataEnum) -> TokenStream {
    let is_numbers = item.variants.iter().all(|m| m.discriminant.is_some());
    let is_labels = item
        .variants
        .iter()
        .all(|m| m.discriminant.is_none() && m.fields.is_empty());

    let vaerdi_name = format_ident!("vaerdi");

    if is_numbers {
        let variants = item.variants.iter().map(|item| {
            let (_, expr) = item.discriminant.as_ref().unwrap();
            let name = &item.ident;
            quote!(#expr => Ok(#enum_name::#name))
        });
        quote!(
            

            impl #vaerdi_name::convert::FromValue for #enum_name {
                type Error = #vaerdi_name::ConvertError;
                fn from_value(from: #vaerdi_name::Value) -> Result<#enum_name, Self::Error> {
                    let Some(number) = from.as_number() else {
                        return Err(#vaerdi_name::ConvertError::invalid_type(#vaerdi_name::Type::I32, from.get_type()));
                    };

                    match number.as_i32() {
                        #(#variants),*,
                        v => {
                            return Err(#vaerdi_name::ConvertError::unknown(format!("invalid variant: {}",v)));
                        }
                    }
                }
            }

        )
        .into()
    } else if is_labels {
        let variants = item.variants.iter().map(|item| {
            let name = &item.ident;
            let name_str = item.ident.to_string();
            quote!(#name_str => Ok(#enum_name::#name))
        });
        quote!(
            impl #vaerdi_name::convert::FromValue for #enum_name {
                type Error = #vaerdi_name::ConvertError;
                fn from_value(from: #vaerdi_name::Value) -> Result<#enum_name, Self::Error> {
                    let Some(name) = from.as_string() else {
                        return Err(#vaerdi_name::ConvertError::invalid_type(#vaerdi_name::Type::String, from.get_type()));
                    };

                    match &**name {
                        #(#variants),*,
                        v => {
                            return Err(#vaerdi_name::ConvertError::unknown_variant(v));
                        }
                    }
                }
            }
        )
        .into()
    } else {
        let variants = item.variants.iter().map(|var| {
            let name = &var.ident;
            let name_str = var.ident.to_string();

            if var.fields.is_empty() {
                quote!(#name_str => {
                    Ok(#enum_name::#name)
                })
            } else {
                if var.fields.len() == 1 {
                    let field = var.fields.iter().next().unwrap();
                    let ty = &field.ty;
                    if field.ident.is_none() {
                        return quote!(#name_str => {
                            let Some(content) = map.remove("content") else {
                                return Err(#vaerdi_name::ConvertError::unknown("field 'content' not found"))
                            };
                            Ok(#enum_name::#name(<#ty as #vaerdi_name::convert::FromValue>::from_value(content)?))
                        });
                    }
                }

                let fields = var.fields.iter().map(|m| {
                    let field_name = m.ident.as_ref().unwrap();
                    let field_name_str = field_name.to_string();
                    let ty = &m.ty;

                    quote!(
                        #field_name: match content.remove(#field_name_str) {
                            Some(m) => <#ty as #vaerdi_name::convert::FromValue>::from_value(m)?,
                            None => {
                                return Err(#vaerdi_name::ConvertError::unknown("field not found"))

                            }
                        }
                    )
                });

                quote!(
                    #name_str => {
                        let Some(mut content) = map.remove("content") else {
                            return Err(#vaerdi_name::ConvertError::unknown("field 'content' not found"))
                        };

                        let Some(mut content) = content.as_map_mut() else {
                            return Err(#vaerdi_name::ConvertError::invalid_type(#vaerdi_name::Type::Map, content.get_type()));
                        };

                        Ok(#enum_name::#name {
                            #(#fields),*,
                        })

                    }
                )
            }
        });

        let (ty, imp, wh) = generics.split_for_impl();

        quote!(


            impl #imp #vaerdi_name::convert::FromValue for #enum_name #ty #wh {
                type Error = #vaerdi_name::ConvertError;
                fn from_value(mut from: #vaerdi_name::Value) -> Result<#enum_name #ty, Self::Error> {
                    let Some(mut map) = from.as_map_mut() else {
                        return Err(#vaerdi_name::ConvertError::invalid_type(#vaerdi_name::Type::Map, from.get_type()))
                    };

                    let Some(ty) = map.get("type") else {
                        return Err(#vaerdi_name::ConvertError::unknown("field 'type' not found"))
                    };

                    let Some(ty_str) = ty.as_string() else {
                        return Err(#vaerdi_name::ConvertError::invalid_type(#vaerdi_name::Type::String, ty.get_type()))
                    };

                    match ty_str.as_str() {
                        #(#variants),*,
                        v => {
                            return Err(#vaerdi_name::ConvertError::unknown_variant(v));
                        }
                    }
                }
            }

        )
        .into()
    }
}
