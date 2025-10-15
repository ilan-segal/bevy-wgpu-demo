use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

#[proc_macro_derive(SpatiallyMapped3d)]
pub fn derive_spatial_3d(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let expanded = match input.data {
        Data::Struct(ref data_struct) => {
            let syn::Fields::Unnamed(ref fields) = data_struct.fields else {
                return syn::Error::new_spanned(
                    name,
                    "SpatiallyMapped can only be derived for tuple structs.",
                )
                .to_compile_error()
                .into();
            };

            if fields.unnamed.len() != 1 {
                return syn::Error::new_spanned(
                    name,
                    "SpatiallyMapped can only be derived for tuple structs with exactly one field.",
                )
                .to_compile_error()
                .into();
            }

            let inner_ty = &fields.unnamed.first().unwrap().ty;

            quote! {
                impl SpatiallyMapped<3> for #name
                where
                    #inner_ty: SpatiallyMapped<3>,
                {
                    type Item = #inner_ty::Item;
                    type Index = #inner_ty::Index;

                    fn at_pos(&self, pos: [usize; Self::DIM]) -> &Self::Item {
                        self.0.at_pos(pos)
                    }
                }
            }
        }
        _ => syn::Error::new_spanned(
            name,
            "SpatiallyMapped can only be derived for tuple structs.",
        )
        .to_compile_error()
        .into(),
    };

    expanded.into()
}
