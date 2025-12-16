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

    let font_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join(path.value());

    if !font_path.exists() {
        return syn::Error::new(
            path.span(),
            format!("Font file does not exist at {}", font_path.display()),
        )
        .to_compile_error()
        .into();
    }

    let size_value = size.base10_digits();

    let chars_value = chars.value();
    let unicode_code_points = chars_to_unicode_code_points(&chars_value);

    let bdf_file_path: PathBuf = font_path.with_extension("bdf");
    let output = Command::new("otf2bdf")
        .arg("-p")
        .arg(size_value)
        .arg("-l")
        .arg(&unicode_code_points)
        .arg(&font_path)
        .output()
        .expect("Failed to run otf2bdf")
        .stdout;

    fs::write(&bdf_file_path, &output)
        .expect("Failed to write .bdf file");

    let bdfconv_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tools/bdfconv/bdfconv");

    let output = Command::new(bdfconv_path)
        .arg("-f")
        .arg("1")
        .arg("-m")
        .arg("32-127")
        .arg("-binary")
        .arg(&bdf_file_path)
        .output()
        .expect("Failed to run bdfconv")
        .stdout;

    fs::remove_file(bdf_file_path).expect("Failed to remove temporary .bdf file");

    let byte_literal = Literal::byte_string(&output);

    let struct_name = Ident::new(
        name.to_string().as_str(),
        name.span(),
    );

    let expanded = quote! {
        pub struct #struct_name {}

        impl u8g2_fonts::Font for #struct_name {
            const DATA: &'static [u8] = #byte_literal;
        }
    };

    expanded.into()
}

fn chars_to_unicode_code_points(chars: &str) -> String {
    chars.chars().map(|c| (c as u32).to_string()).collect::<Vec::<String>>().join(" ")
}
