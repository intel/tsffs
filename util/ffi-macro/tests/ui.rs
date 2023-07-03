#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    // t.compile_fail("tests/ui/*.rs");
    t.compile_fail("tests/ui/test_missing_params.rs");
}
