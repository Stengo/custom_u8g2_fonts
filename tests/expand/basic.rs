use custom_u8g2_fonts::u8g2_font;

u8g2_font!(
    path = "../../../../tests/fonts/Nunito.ttf",
    name = Sample,
    size = 12,
    chars = "0123456789:"
);

fn main() {
    let _ = Sample12 {};
}
