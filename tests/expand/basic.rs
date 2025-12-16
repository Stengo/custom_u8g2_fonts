use custom_u8g2_fonts::u8g2_font;

u8g2_font!(
    path = "../fonts/Nunito.ttf",
    name = Sample,
    size = 12,
    weight = "Bold"
);

fn main() {
    let _ = SampleBold12 {};
}
