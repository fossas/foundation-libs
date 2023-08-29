use snippets::{language::c99_tc3, Extractor, Support};

#[test]
fn hello_world() {
    crate::tracing::setup();

    let content = include_bytes!("testdata/hello_world.c");
    let support = c99_tc3::Extractor::support(content).expect("must set up parser");
    assert_eq!(support, Support::Full);
}

#[test]
fn hello_world_cpp() {
    crate::tracing::setup();

    let content = include_bytes!("testdata/hello_world.cpp");
    let support = c99_tc3::Extractor::support(content).expect("must set up parser");
    assert_eq!(support, Support::None);
}
