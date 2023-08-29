use snippets::{
    language::c99_tc3, Extractor, Kind, Location, Metadata, Method, Options, PrettyBuffer, Snippet,
    Transforms,
};
use tap::Pipe;

use crate::{assert_snippets_eq, must};

#[test]
fn hello_world() {
    crate::tracing::setup();

    let content = include_bytes!("testdata/hello_world.c");
    let opts = Options::new(Kind::Full, Transforms::none());
    let extract = c99_tc3::Extractor::extract(&opts, content).expect("must set up parser");

    let expected = vec![Snippet::new(
        Metadata::new(Kind::Full, Method::Raw, Location::from(21..74)),
        PrettyBuffer::new_base64("RjpGXHcd2yCz04Q+BXBT3w65hnLBbXZE+zcmBW4OPw0").pipe(must),
    )];

    assert_snippets_eq!(content => extract, expected);
}

#[test]
fn hello_world_cpp() {
    crate::tracing::setup();

    let content = include_bytes!("testdata/hello_world.cpp");
    let opts = Options::new(Kind::Full, Transforms::none());
    let extract = c99_tc3::Extractor::extract(&opts, content).expect("must set up parser");

    assert!(extract.is_empty());
}
