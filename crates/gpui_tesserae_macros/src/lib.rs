use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Expr, Meta, Type, parse_macro_input};

#[proc_macro_derive(IntoThemeField, attributes(theme, field))]
pub fn into_theme_field_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let variants = if let Data::Enum(data_enum) = input.data {
        data_enum.variants
    } else {
        panic!("#[derive(IntoThemeField)] can only be used on enums");
    };

    let field_type = get_enum_field_type(&input.attrs);

    // Example: generate a method that returns the theme string for each variant
    let variant_matches = variants.iter().map(|variant| {
        let ident = &variant.ident;
        let field_path = get_theme_expr(&variant.attrs);

        quote! {
            #name::#ident => &theme.#field_path,
        }
    });

    let expanded = quote! {
        impl #name {
            pub fn resolve<'a>(&'a self, theme: &'a gpui_tesserae_theme::Theme) -> &'a #field_type {
                match self {
                    #(#variant_matches)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

fn get_theme_expr(attrs: &[Attribute]) -> Expr {
    let theme_attr = attrs
        .iter()
        .find(|attr| attr.path().is_ident("theme"))
        .expect("Variant is missing #[theme(...)] attribute");

    match &theme_attr.meta {
        Meta::List(list) => {
            // Parse tokens as an arbitrary expression
            syn::parse2(list.tokens.clone()).expect("Expected an expression inside #[theme(...)]")
        }
        _ => panic!("#[theme(...)] must be a list"),
    }
}

fn get_enum_field_type(attrs: &[Attribute]) -> Type {
    let field_attr = attrs
        .iter()
        .find(|attr| attr.path().is_ident("field"))
        .expect("Enum is missing #[field(...)] attribute");

    match &field_attr.meta {
        Meta::List(meta_list) => {
            // Parse the inner tokens as a type
            syn::parse2::<Type>(meta_list.tokens.clone())
                .expect("#[field(...)] must contain a valid type")
        }
        _ => panic!("#[field(...)] must be a list, like #[field(String)]"),
    }
}
