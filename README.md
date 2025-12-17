# custom_u8g2_fonts

This crate introduces the helper macro `u8g2_font!` for [u8g2-fonts](https://github.com/Finomnis/u8g2-fonts), which allows you to automatically convert any `.ttf` or `.otf` file into a usable [Font](https://docs.rs/u8g2-fonts/latest/u8g2_fonts/trait.Font.html) during compile time.

## Usage

Before starting, make sure that you have [otf2bdf](https://github.com/jirutka/otf2bdf) installed and added to your `PATH`. On macOS, you can install it via [brew](https://formulae.brew.sh/formula/otf2bdf).

Within your project, all you have to do is provide the path to the font relative to your project root, a name for the font, the size it should be rendered at, and a string containing all of the characters you would like to convert.

```Rust
use custom_u8g2_fonts::u8g2_font;
use u8g2_fonts::FontRenderer;

u8g2_font!(
    path = "./src/fonts/Nunito-ExtraBold.ttf",
    name = LargeNumbers,
    size = 30,
    chars = "0123456789"
);

let renderer = FontRenderer::new::<LargeNumbers>();
```

## Acknowledgements

This crate is merely a convenience wrapper around the excellent work done by [jirutka](https://github.com/jirutka) on [otf2bdf](https://github.com/jirutka/otf2bdf) and [olikraus](https://github.com/olikraus) on [u8g2/bdfconv](https://github.com/olikraus/u8g2) and is only useful because of the lovely conversion [Finomnis](https://github.com/Finomnis) did with [u8g2-fonts](https://github.com/Finomnis/u8g2-fonts).

A huge thank you to all of you for your contributions to the open source ecosystem!
