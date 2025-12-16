use std::{env, fs};
use std::path::{Path, PathBuf};
use std::process::Command;
use proc_macro::TokenStream;
use proc_macro2::Literal;
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
    chars: LitStr,
}

impl Parse for FontInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut path = None;
        let mut name = None;
        let mut size = None;
        let mut chars = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "path" => path = Some(input.parse()?),
                "name" => name = Some(input.parse()?),
                "size" => size = Some(input.parse()?),
                "chars" => chars = Some(input.parse()?),
                _ => return Err(input.error("Unknown argument")),
            }

            let _ = input.parse::<Token![,]>();
        }

        Ok(FontInput {
            path: path.ok_or_else(|| input.error("Missing `path`"))?,
            name: name.ok_or_else(|| input.error("Missing `name`"))?,
            size: size.ok_or_else(|| input.error("Missing `size`"))?,
            chars: chars.ok_or_else(|| input.error("Missing `chars`"))?,
        })
    }
}

#[proc_macro]
pub fn u8g2_font(input: TokenStream) -> TokenStream {
    let FontInput {
        path,
        name,
        size,
        chars,
    } = parse_macro_input!(input as FontInput);

    match generate_font_data(path, name, size, chars) {
        Ok(token_stream) => token_stream,
        Err(error) => error.to_compile_error().into(),
    }
}

fn generate_font_data(
    path: LitStr,
    name: Ident,
    size: LitInt,
    chars: LitStr,
) -> syn::Result<TokenStream> {
    let font_path = resolve_font_path(&path)?;
    let size_value = size.base10_digits();
    let unicode_code_points = chars_to_unicode_code_points(&chars.value());
    let bdf_file_path = font_path.with_extension("bdf");

    let bdf_output = generate_bdf_from_otf(&font_path, size_value, &unicode_code_points)?;
    fs::write(&bdf_file_path, bdf_output).map_err(|e| syn::Error::new(path.span(), format!("Failed to write .bdf file: {}", e)))?;

    let font_bytes = generate_font_bytes_from_bdf(&bdf_file_path)?;
    fs::remove_file(&bdf_file_path).map_err(|e| syn::Error::new(path.span(), format!("Failed to remove temporary .bdf file: {}", e)))?;

    generate_output_tokens(&name, &font_bytes)
}

fn resolve_font_path(path_lit: &LitStr) -> syn::Result<PathBuf> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map_err(|e| syn::Error::new(path_lit.span(), format!("CARGO_MANIFEST_DIR not set: {}", e)))?;
    let font_path = PathBuf::from(manifest_dir).join(path_lit.value());

    if !font_path.exists() {
        return Err(syn::Error::new(
            path_lit.span(),
            format!("Font file does not exist at {}", font_path.display()),
        ));
    }
    Ok(font_path)
}

fn generate_bdf_from_otf(
    font_path: &Path,
    size_value: &str,
    unicode_code_points: &str,
) -> syn::Result<Vec<u8>> {
    Command::new("otf2bdf")
        .arg("-p")
        .arg(size_value)
        .arg("-l")
        .arg(unicode_code_points)
        .arg(font_path)
        .output()
        .map_err(|e| syn::Error::new_spanned(font_path.to_str(), format!("Failed to run otf2bdf: {}", e)))
        .map(|output| output.stdout)
}

fn generate_font_bytes_from_bdf(bdf_file_path: &Path) -> syn::Result<Vec<u8>> {
    let bdfconv_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tools/bdfconv/bdfconv");

    Command::new(bdfconv_path)
        .arg("-f")
        .arg("1")
        .arg("-m")
        .arg("32-127")
        .arg("-binary")
        .arg(bdf_file_path)
        .output()
        .map_err(|e| syn::Error::new_spanned(bdf_file_path.to_str(), format!("Failed to run bdfconv: {}", e)))
        .map(|output| output.stdout)
}

fn generate_output_tokens(name: &Ident, font_bytes: &[u8]) -> syn::Result<TokenStream> {
    let byte_literal = Literal::byte_string(font_bytes);
    let expanded = quote! {
        pub struct #name {}

        impl u8g2_fonts::Font for #name {
            const DATA: &'static [u8] = #byte_literal;
        }
    };
    Ok(expanded.into())
}

fn chars_to_unicode_code_points(chars: &str) -> String {
    chars.chars().map(|c| (c as u32).to_string()).collect::<Vec::<String>>().join(" ")
}
