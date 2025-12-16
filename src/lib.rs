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

#[derive(Debug)]
enum CharacterSet {
    String(String),
    Numbers,
    LowerCase,
    UpperCase,
    Punctuation,
}

impl Parse for CharacterSet {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Ident) {
            let ident: Ident = input.parse()?;
            match ident.to_string().as_str() {
                "Numbers" => return Ok(CharacterSet::Numbers),
                "LowerCase" => return Ok(CharacterSet::LowerCase),
                "UpperCase" => return Ok(CharacterSet::UpperCase),
                "Punctuation" => return Ok(CharacterSet::Punctuation),
                _ => return Err(input.error(format!("Unknown character set identifier: {}", ident))),
            }
        } 
        
        if input.peek(LitStr) {
            let lit_str: LitStr = input.parse()?;
            return Ok(CharacterSet::String(lit_str.value()));
        }

        Err(input.error("Expected an identifier (Numbers, LowerCase, UpperCase, Punctuation) or a string literal (\"abc\")."))
    }
}

struct FontInput {
    path: LitStr,
    name: Ident,
    size: LitInt,
    specs: Vec<CharacterSet>,
}

impl Parse for FontInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut path = None;
        let mut name = None;
        let mut size = None;
        let mut specs = Vec::new();

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            match ident.to_string().as_str() {
                "path" => path = Some(input.parse()?),
                "name" => name = Some(input.parse()?),
                "size" => size = Some(input.parse()?),
                "chars" => {
                    let list = input.parse_terminated(CharacterSet::parse, Token![,])?;
                    specs.extend(list.into_iter());
                },
                _ => return Err(input.error("Unknown argument")),
            }

            let _ = input.parse::<Token![,]>();
        }

        Ok(FontInput {
            path: path.ok_or_else(|| input.error("Missing `path`"))?,
            name: name.ok_or_else(|| input.error("Missing `name`"))?,
            size: size.ok_or_else(|| input.error("Missing `size`"))?,
            specs,
        })
    }
}

#[proc_macro]
pub fn u8g2_font(input: TokenStream) -> TokenStream {
    let FontInput {
        path,
        name,
        size,
        specs,
    } = parse_macro_input!(input as FontInput);

    match generate_font_data(path, name, size, specs) {
        Ok(token_stream) => token_stream,
        Err(error) => error.to_compile_error().into(),
    }
}

fn generate_font_data(
    path: LitStr,
    name: Ident,
    size: LitInt,
    specs: Vec<CharacterSet>,
) -> syn::Result<TokenStream> {
    let font_path = resolve_font_path(&path)?;
    let size_value = size.base10_digits();
    
    let unicode_code_points = specs_to_unicode_code_points(&specs);

    let bdf_file_path = font_path.with_extension("bdf");

    let bdf_output = generate_bdf_from_otf(&font_path, size_value, &unicode_code_points)?;
    fs::write(&bdf_file_path, bdf_output).map_err(|e| syn::Error::new(path.span(), format!("Failed to write .bdf file: {}", e)))?;

    let font_bytes = generate_font_bytes_from_bdf(&bdf_file_path, &unicode_code_points)?;
    fs::remove_file(&bdf_file_path).map_err(|e| syn::Error::new(path.span(), format!("Failed to remove temporary .bdf file: {}", e)))?;

    generate_output_tokens(&name, &font_bytes)
}

fn specs_to_unicode_code_points(specs: &[CharacterSet]) -> Vec<u32> {
    let mut collected_chars = std::collections::BTreeSet::new();

    for spec in specs {
        match spec {
            CharacterSet::String(s) => {
                s.chars().for_each(|c| { collected_chars.insert(c); });
            }
            CharacterSet::Numbers => {
                ('0'..='9').for_each(|c| { collected_chars.insert(c); });
            }
            CharacterSet::LowerCase => {
                ('a'..='z').for_each(|c| { collected_chars.insert(c); });
            }
            CharacterSet::UpperCase => {
                ('A'..='Z').for_each(|c| { collected_chars.insert(c); });
            }
            CharacterSet::Punctuation => {
                ".,'\"?!:;()-".chars().for_each(|c| { collected_chars.insert(c); });
            }
        }
    }
    
    collected_chars.iter().map(|&c| (c as u32)).collect::<Vec<u32>>()
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
    unicode_code_points: &Vec<u32>,
) -> syn::Result<Vec<u8>> {
    let output = Command::new("otf2bdf")
        .arg("-p")
        .arg(size_value)
        .arg("-l")
        .arg(unicode_code_points.iter().map(|c| c.to_string()).collect::<Vec<String>>().join(" "))
        .arg(font_path)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                syn::Error::new_spanned(font_path.to_str(), "Failed to run `otf2bdf`. Is it installed and in your PATH?")
            } else {
                syn::Error::new_spanned(font_path.to_str(), format!("Failed to run `otf2bdf`: {}", e))
            }
        })?;

    if !output.status.success() {
        let stderr_msg = String::from_utf8_lossy(&output.stderr);
        return Err(syn::Error::new_spanned(
            font_path.to_str(),
            format!("`otf2bdf` command failed: {}", stderr_msg.trim())
        ));
    }

    Ok(output.stdout)
}

fn generate_font_bytes_from_bdf(bdf_file_path: &Path, unicode_code_points: &Vec<u32>) -> syn::Result<Vec<u8>> {
    let bdfconv_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tools/bdfconv/bdfconv");

    let output = Command::new(&bdfconv_path)
        .arg("-f")
        .arg("1")
        .arg("-m")
        .arg(unicode_code_points.iter().map(|c| c.to_string()).collect::<Vec<String>>().join(","))
        .arg("-binary")
        .arg(bdf_file_path)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                 syn::Error::new_spanned(bdf_file_path.to_str(), format!("Failed to run `bdfconv` at '{}'. Check that the executable exists.", bdfconv_path.display()))
            } else {
                 syn::Error::new_spanned(bdf_file_path.to_str(), format!("Failed to run `bdfconv`: {}", e))
            }
        })?;
    
    if !output.status.success() {
        let stderr_msg = String::from_utf8_lossy(&output.stderr);
        return Err(syn::Error::new_spanned(
            bdf_file_path.to_str(),
            format!("`bdfconv` command failed: {}", stderr_msg.trim())
        ));
    }

    Ok(output.stdout)
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
