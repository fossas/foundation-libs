use snippets::{
    language::c99_tc3, Extractor, Kind, Location, Metadata, Method, Options, PrettyBuffer, Snippet,
    Transforms,
};
use tap::Pipe;

use crate::{assert_snippets_eq, must};

#[test]
fn full_raw_hello_world() {
    crate::tracing::setup();

    let content = include_bytes!("testdata/c99_tc3/hello_world.c");
    let opts = Options::new(Kind::Full, Transforms::none());
    let extract = c99_tc3::Extractor::extract(&opts, content).expect("must set up parser");

    let expected = vec![Snippet::new(
        Metadata::new(Kind::Full, Method::Raw, Location::from(21..74)),
        PrettyBuffer::new_base64("RjpGXHcd2yCz04Q+BXBT3w65hnLBbXZE+zcmBW4OPw0").pipe(must),
    )];

    assert_snippets_eq!(content => extract, expected);
}

#[test]
fn full_raw_hello_world_syntax_error() {
    crate::tracing::setup();

    let content = include_bytes!("testdata/c99_tc3/hello_world_error.c");
    let opts = Options::new(Kind::Full, Transforms::none());
    let extract = c99_tc3::Extractor::extract(&opts, content).expect("must set up parser");

    let expected = vec![Snippet::new(
        Metadata::new(Kind::Full, Method::Raw, Location::from(21..=68)),
        PrettyBuffer::new_base64("7SCsdDPxPL3YWRHA2DO9AWfKFv11QWeNzNYF3PLSnVQ").pipe(must),
    )];

    assert_snippets_eq!(content => extract, expected);
}
