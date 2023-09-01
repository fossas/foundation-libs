use snippets::{
    language::c99_tc3, text, Extractor, Kind, Location, Metadata, Method, Options, Snippet, Target,
    Transforms,
};

use crate::{assert_snippets_eq, include_str_lf};

#[test]
fn full_raw_hello_world() {
    crate::tracing::setup();

    let content = include_str_lf!("testdata/c99_tc3/hello_world.c");
    let opts = Options::new(Target::Function, Kind::Full, Transforms::none());
    let extract = c99_tc3::Extractor::extract(&opts, &content).expect("must set up parser");

    let expected = vec![Snippet::new(
        Metadata::new(Kind::Full, Method::Raw, Location::from(21..74)),
        text::Buffer::base64("RjpGXHcd2yCz04Q+BXBT3w65hnLBbXZE+zcmBW4OPw0").unwrap(),
    )];

    assert_snippets_eq!(content.as_bytes() => extract, expected);
}

#[test]
fn full_raw_hello_world_crlf_lf() {
    crate::tracing::setup();

    // This test runs on both Windows and other platforms, so it normalizes
    // to \n regardless of the actual example file and then expands that back to \r\n.
    //
    // On non-Windows the first replace will just effectively do nothing.
    let content_lf = include_str!("testdata/c99_tc3/hello_world.c").replace("\r\n", "\n");
    let content_crlf = content_lf.replace('\n', "\r\n");

    let opts = Options::new(Target::Function, Kind::Full, Transforms::none());
    let extract_lf = c99_tc3::Extractor::extract(&opts, &content_lf).unwrap();
    let extract_crlf = c99_tc3::Extractor::extract(&opts, &content_crlf).unwrap();

    // Even though the fingerprints themselves are normalized, they'll still be at different byte offsets.
    let shared_fp = text::Buffer::base64("RjpGXHcd2yCz04Q+BXBT3w65hnLBbXZE+zcmBW4OPw0").unwrap();
    let expected_lf = vec![Snippet::new(
        Metadata::new(Kind::Full, Method::Raw, Location::from(21..74)),
        shared_fp.clone(),
    )];
    let expected_crlf = vec![Snippet::new(
        Metadata::new(Kind::Full, Method::Raw, Location::from(24..80)),
        shared_fp,
    )];

    assert_snippets_eq!(content_lf.as_bytes() => extract_lf, expected_lf);
    assert_snippets_eq!(content_crlf.as_bytes() => extract_crlf, expected_crlf);
}

#[test]
fn full_raw_hello_world_syntax_error() {
    crate::tracing::setup();

    let content = include_str_lf!("testdata/c99_tc3/hello_world_error.c");
    let opts = Options::new(Target::Function, Kind::Full, Transforms::none());
    let extract = c99_tc3::Extractor::extract(&opts, &content).expect("must set up parser");

    let expected = vec![Snippet::new(
        Metadata::new(Kind::Full, Method::Raw, Location::from(21..=68)),
        text::Buffer::base64("7SCsdDPxPL3YWRHA2DO9AWfKFv11QWeNzNYF3PLSnVQ").unwrap(),
    )];

    assert_snippets_eq!(content.as_bytes() => extract, expected);
}

#[test]
fn signature_raw_hello_world() {
    crate::tracing::setup();

    let content = include_str_lf!("testdata/c99_tc3/hello_world.c");
    let opts = Options::new(Target::Function, Kind::Signature, Transforms::none());
    let extract = c99_tc3::Extractor::extract(&opts, &content).expect("must set up parser");

    let expected = vec![Snippet::new(
        Metadata::new(Kind::Signature, Method::Raw, Location::from(21..31)),
        text::Buffer::base64("OFxpLQXYGQzLjWud7D9ZLoq6Agzu9eaH/38i58yqZJs").unwrap(),
    )];

    assert_snippets_eq!(content.as_bytes() => extract, expected);
}
