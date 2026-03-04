use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(WrapperType)]
pub fn wrapper_type_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let type_name = &input.ident;

    match input.data {
        syn::Data::Struct(ref data) => match data.fields {
            syn::Fields::Unnamed(ref fields) if fields.unnamed.len() == 1 => {}
            _ => panic!("wrappertype supports only tuple structs with one field"),
        },
        _ => panic!("wrappertype supports only structs"),
    }

    let field_name = type_name.to_string().to_snake_case();

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expanded = quote! {
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

        impl<'de> #impl_generics serde::Deserialize<'de>
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
    };

    TokenStream::from(expanded)
}
