#[test]
fn compile() {
    let t = trybuild::TestCases::new();
    t.pass("tests/expand/basic.rs");
}
