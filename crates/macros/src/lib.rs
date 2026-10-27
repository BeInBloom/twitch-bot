use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input, parse_quote};

#[proc_macro_derive(WrapperType)]
pub fn wrapper_type_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_wrapper_type(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(SecretDebug)]
pub fn secret_debug_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let type_name = &input.ident;
    let masked = format!("{type_name}(***)");
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics ::std::fmt::Debug for #type_name #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(#masked)
            }
        }
    }
    .into()
}

fn expand_wrapper_type(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    ensure_single_field_tuple_struct(input)?;

    let type_name = &input.ident;
    let field_name = type_name.to_string().to_snake_case();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let mut deserialize_generics = input.generics.clone();
    deserialize_generics.params.insert(0, parse_quote!('de));
    let (deserialize_impl_generics, _, _) = deserialize_generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics TryFrom<String>
            for #type_name #ty_generics
            #where_clause
        {
            type Error = macros_core::WrapperValidationError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.is_empty() {
                    return Err(
                        macros_core::WrapperValidationError::new(
                            #field_name,
                            "cannot be empty",
                        )
                    );
                }
                Ok(Self(value))
            }
        }

        impl #deserialize_impl_generics serde::Deserialize<'de>
            for #type_name #ty_generics
            #where_clause
        {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let s = <String as serde::Deserialize>::deserialize(deserializer)?;
                Self::try_from(s).map_err(serde::de::Error::custom)
            }
        }

        impl #impl_generics #type_name #ty_generics #where_clause {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    })
}

fn ensure_single_field_tuple_struct(input: &DeriveInput) -> syn::Result<()> {
    match &input.data {
        Data::Struct(data) if matches!(&data.fields, Fields::Unnamed(fields) if fields.unnamed.len() == 1) => {
            Ok(())
        }
        _ => Err(syn::Error::new_spanned(
            &input.ident,
            "WrapperType supports only tuple structs with one field",
        )),
    }
}
