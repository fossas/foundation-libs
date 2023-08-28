use snippets::{language::c99_tc3, Extractor, Support};

#[test]
fn hello_world() {
    let content = include_bytes!("testdata/hello_world.c");
    let support = c99_tc3::Extractor::support(content).expect("must parse input");
    assert_eq!(support, Support::Full);
}
