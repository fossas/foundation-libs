use snippets::{
    language::cpp_98,
    Extractor, Kind, Kinds, Location, Metadata, Options, Snippet, Target, Targets, Transforms,
};

use crate::include_str_lf;

#[test]
fn smoke_test() {
    crate::tracing::setup();

    let target = Targets::default();
    let kind = Kinds::default();
    let transform = Transforms::default();

    let content = include_str_lf!("testdata/cpp_98/smoke_test.cc");
    let opts = Options::new(target, kind, transform).disable_raw();
    let extract = cpp_98::Extractor::extract(&opts, content).expect("extract snippets");
    assert!(!extract.is_empty(), "must have extracted snippets");
}

#[test]
fn functions_in_namespaces_full() {
    crate::tracing::setup();

    let kind = Kind::Full;
    let transform = None;
    let helloworld_span = Location::from(99..206);

    let content = include_str_lf!("testdata/cpp_98/bare_function_in_namespace.cc");
    let opts = Options::new(Target::Function, kind, transform);

    let extract = cpp_98::Extractor::extract(&opts, &content).expect("extract snippets");
    let expected = vec![Snippet::from(
        Metadata::new(kind, transform.into(), helloworld_span),
        helloworld_span.extract_from(content.as_bytes()),
    )];

    assert_eq!(extract, expected);
}

#[test]
fn function_in_namespace_signature() {
    crate::tracing::setup();

    let kind = Kind::Signature;
    let transform = None;
    let helloworld_span = Location::from(99..124);

    let content = include_str_lf!("testdata/cpp_98/bare_function_in_namespace.cc");
    let opts = Options::new(Target::Function, kind, transform);

    let extract = cpp_98::Extractor::extract(&opts, &content).expect("extract snippets");
    let expected = vec![
        Snippet::from(
            Metadata::new(kind, transform.into(), helloworld_span),
            helloworld_span.extract_from(content.as_bytes()),
        ),
    ];

    assert_eq!(extract, expected);
}
