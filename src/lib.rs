#![feature(proc_macro_span)]

use std::path::PathBuf;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    Ident, LitInt, LitStr, Result, Token,
};

struct FontInput {
    path: LitStr,
    name: Ident,
    size: LitInt,
    weight: LitStr,
}

impl Parse for FontInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut path = None;
        let mut name = None;
        let mut size = None;
        let mut weight = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "path" => path = Some(input.parse()?),
                "name" => name = Some(input.parse()?),
                "size" => size = Some(input.parse()?),
                "weight" => weight = Some(input.parse()?),
                _ => return Err(input.error("Unknown argument")),
            }

            let _ = input.parse::<Token![,]>();
        }

        Ok(FontInput {
            path: path.ok_or_else(|| input.error("Missing `path`"))?,
            name: name.ok_or_else(|| input.error("Missing `name`"))?,
            size: size.ok_or_else(|| input.error("Missing `size`"))?,
            weight: weight.ok_or_else(|| input.error("Missing `weight`"))?,
        })
    }
}

#[proc_macro]
pub fn u8g2_font(input: TokenStream) -> TokenStream {
    let FontInput {
        path,
        name,
        size,
        weight,
    } = parse_macro_input!(input as FontInput);

    let span = proc_macro::Span::call_site();
    let source_file = span.source_file();
    let source_path = source_file.path();

    let base_dir = source_path
        .parent()
        .expect("Source file should have a parent directory");

    let full_path: PathBuf = base_dir.join(path.value());

    if !full_path.exists() {
        return syn::Error::new(
            path.span(),
            format!(
                "Font file does not exist (relative to {}): {}",
                source_path.display(),
                full_path.display(),
            ),
        )
        .to_compile_error()
        .into();
    }

    let size_value = size.base10_digits();
    let weight_value = weight.value();

    let struct_name = Ident::new(
        &format!("{name}{weight_value}{size_value}"),
        name.span(),
    );

    let expanded = quote! {
        pub struct #struct_name {}

        impl u8g2_fonts::Font for #struct_name {
            const DATA: &'static [u8] = include_bytes!(#path);
        }
    };

    expanded.into()
}
