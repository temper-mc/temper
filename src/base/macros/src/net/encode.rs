use crate::helpers::{get_derive_attributes, StructInfo};
use crate::net::packets::get_packet_details_from_attributes;
use crate::static_loading::packets::PacketBoundiness;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Fields};

// Generate packet ID encoding snippets
fn generate_packet_id_snippets(packet_id: Option<u8>) -> proc_macro2::TokenStream {
    if let Some(id) = packet_id {
        quote! {
            <temper_codec::net_types::var_int::VarInt as temper_codec::encode::NetEncode>::encode(&#id.into(), writer, &temper_codec::encode::NetEncodeOpts::None)?;
        }
    } else {
        quote! {}
    }
}

// Generate field encoding expressions for structs
fn generate_field_encoders(fields: &syn::Fields) -> proc_macro2::TokenStream {
    let encode_fields = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_ty = &field.ty;
        quote! {
            <#field_ty as temper_codec::encode::NetEncode>::encode(&self.#field_name, writer, &temper_codec::encode::NetEncodeOpts::None)?;
        }
    });
    quote! { #(#encode_fields)* }
}

// Generate enum variant encoding using static dispatch
fn generate_enum_encoders(data: &syn::DataEnum) -> proc_macro2::TokenStream {
    let variants: Vec<_> = data.variants.iter().map(|variant| {
        let variant_ident = &variant.ident;

        match &variant.fields {
            Fields::Named(fields) => {
                let field_idents: Vec<_> = fields.named.iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();
                let field_tys: Vec<_> = fields.named.iter()
                    .map(|f| &f.ty)
                    .collect();

                quote! {
                    Self::#variant_ident { #(#field_idents),* } => {
                        #(
                            <#field_tys as temper_codec::encode::NetEncode>::encode(#field_idents, writer, &temper_codec::encode::NetEncodeOpts::None)?;
                        )*
                    }
                }
            }
            Fields::Unnamed(fields) => {
                let field_names: Vec<_> = (0..fields.unnamed.len())
                    .map(|i| syn::Ident::new(&format!("field{i}"), proc_macro2::Span::call_site()))
                    .collect();
                let field_tys: Vec<_> = fields.unnamed.iter()
                    .map(|f| &f.ty)
                    .collect();

                quote! {
                    Self::#variant_ident(#(#field_names),*) => {
                        #(
                            <#field_tys as temper_codec::encode::NetEncode>::encode(#field_names, writer, &temper_codec::encode::NetEncodeOpts::None)?;
                        )*
                    }
                }
            }
            Fields::Unit => quote! {
                Self::#variant_ident => {}
            },
        }
    }).collect();

    quote! {
        match self {
            #(#variants)*
        }
    }
}

pub(crate) fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let packet_attr = get_derive_attributes(&input, "packet");
    let packet_id_snippet = generate_packet_id_snippets(
        get_packet_details_from_attributes(packet_attr.as_slice(), PacketBoundiness::Clientbound)
            .unzip()
            .1,
    );

    let sync_impl = match &input.data {
        syn::Data::Struct(data) => {
            let field_encoders = generate_field_encoders(&data.fields);

            quote! {
                fn encode<W: std::io::Write>(&self, writer: &mut W, opts: &temper_codec::encode::NetEncodeOpts) -> Result<(),  temper_codec::encode::errors::NetEncodeError> {
                    match opts {
                        temper_codec::encode::NetEncodeOpts::None => {
                            #packet_id_snippet
                            #field_encoders
                        }
                        temper_codec::encode::NetEncodeOpts::WithLength => {
                            let actual_writer = writer;
                            let mut writer = Vec::new();
                            let mut writer = &mut writer;

                            #packet_id_snippet
                            #field_encoders

                            let len: temper_codec::net_types::var_int::VarInt = writer.len().into();
                            <temper_codec::net_types::var_int::VarInt as temper_codec::encode::NetEncode>::encode(&len, actual_writer, &temper_codec::encode::NetEncodeOpts::None)?;
                            actual_writer.write_all(writer)?;
                        }
                        e => unimplemented!("Unsupported option for NetEncode: {:?}", e),
                    }
                    Ok(())
                }
            }
        }
        syn::Data::Enum(data) => {
            let sync_enum_encoder = generate_enum_encoders(data);

            quote! {
                fn encode<W: std::io::Write>(&self, writer: &mut W, opts: &temper_codec::encode::NetEncodeOpts) -> Result<(),  temper_codec::encode::errors::NetEncodeError> {
                    match opts {
                        temper_codec::encode::NetEncodeOpts::None => {
                            #packet_id_snippet
                            #sync_enum_encoder
                        }
                        temper_codec::encode::NetEncodeOpts::WithLength => {
                            let actual_writer = writer;
                            let mut writer = Vec::new();
                            let mut writer = &mut writer;

                            #packet_id_snippet
                            #sync_enum_encoder

                            let len: temper_codec::net_types::var_int::VarInt = writer.len().into();
                            <temper_codec::net_types::var_int::VarInt as temper_codec::encode::NetEncode>::encode(&len, actual_writer, &temper_codec::encode::NetEncodeOpts::None)?;
                            actual_writer.write_all(writer)?;
                        }
                        e => unimplemented!("Unsupported option for NetEncode: {:?}", e),
                    }
                    Ok(())
                }
            }
        }
        _ => unimplemented!("NetEncode can only be derived for structs and enums"),
    };

    let StructInfo {
        struct_name,
        impl_generics,
        ty_generics,
        where_clause,
        lifetime: _lifetime,
        ..
    } = crate::helpers::extract_struct_info(&input, None);

    TokenStream::from(quote! {
        impl #impl_generics temper_codec::encode::NetEncode for #struct_name #ty_generics #where_clause {
            #sync_impl
        }
    })
}
