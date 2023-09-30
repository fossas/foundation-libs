use snippets::{
    language::cpp_98, Extractor, Kind, Kinds, Location, Metadata, Options, Snippet, Target,
    Targets, Transform, Transforms,
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
fn functions_in_namespaces_full_comment_normalized() {
    crate::tracing::setup();

    let kind = Kind::Full;
    let transform = Some(Transform::Comment);
    let helloworld_span = Location::from(99..206);

    let content = include_str_lf!("testdata/cpp_98/bare_function_in_namespace.cc");
    let opts = Options::new(Target::Function, kind, transform).disable_raw();

    let extract = cpp_98::Extractor::extract(&opts, content).expect("extract snippets");
    let expected = vec![Snippet::from(
        Metadata::new(kind, transform.into(), helloworld_span),
        r#"int main() {
      std::cout << "Hello World!";
      return 0;
      
  }"#
        .as_bytes(),
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
    let expected = vec![Snippet::from(
        Metadata::new(kind, transform.into(), helloworld_span),
        helloworld_span.extract_from(content.as_bytes()),
    )];

    assert_eq!(extract, expected);
}

#[test]
fn function_in_class_full_raw() {
    crate::tracing::setup();

    let kind = Kind::Full;
    let transform = None;

    let member_method_span = Location::from(65..154);
    let another_method_span = Location::from(163..260);
    let a_bare_function_span = Location::from(264..380);

    let content = include_str_lf!("testdata/cpp_98/simple_class.cc");
    let opts = Options::new(Target::Function, kind, transform);

    let extract = cpp_98::Extractor::extract(&opts, &content).expect("extract snippets");
    let expected = vec![
        Snippet::from(
            Metadata::new(kind, transform.into(), member_method_span),
            member_method_span.extract_from(content.as_bytes()),
        ),
        Snippet::from(
            Metadata::new(kind, transform.into(), another_method_span),
            another_method_span.extract_from(content.as_bytes()),
        ),
        Snippet::from(
            Metadata::new(kind, transform.into(), a_bare_function_span),
            a_bare_function_span.extract_from(content.as_bytes()),
        ),
    ];

    assert_eq!(extract, expected);
}

#[test]
fn function_in_class_full_comment() {
    crate::tracing::setup();

    let kind = Kind::Full;
    let transform = Some(Transform::Comment);
    let member_method_span = Location::from(65..154);
    let another_method_span = Location::from(163..260);
    let a_bare_function_span = Location::from(264..380);

    let content = include_str_lf!("testdata/cpp_98/simple_class.cc");
    let opts = Options::new(Target::Function, kind, transform).disable_raw();

    let extract = cpp_98::Extractor::extract(&opts, content).expect("extract snippets");
    let expected = vec![
        Snippet::from(
            Metadata::new(kind, transform.into(), member_method_span),
            r#"int member_method(){
      
      return 0;
    }"#
            .as_bytes(),
        ),
        Snippet::from(
            Metadata::new(kind, transform.into(), another_method_span),
            r#"int Foo::another_method() {
    
    return 0;
  }"#
            .as_bytes(),
        ),
        Snippet::from(
            Metadata::new(kind, transform.into(), a_bare_function_span),
            r#"int a_bare_function(  ){
    
    return 0;
  }"#
            .as_bytes(),
        ),
    ];

    assert_eq!(extract, expected);
}

#[test]
fn function_in_class_full_code() {
    crate::tracing::setup();

    let kind = Kind::Full;
    let transform = Some(Transform::Code);
    let member_method_span = Location::from(65..154);
    let another_method_span = Location::from(163..260);
    let a_bare_function_span = Location::from(264..380);

    let content = include_str_lf!("testdata/cpp_98/simple_class.cc");
    let opts = Options::new(Target::Function, kind, transform).disable_raw();

    let extract = cpp_98::Extractor::extract(&opts, content).expect("extract snippets");
    let expected = vec![
        Snippet::from(
            Metadata::new(kind, transform.into(), member_method_span),
            r#"int member_method(){ return 0; }"#.as_bytes(),
        ),
        Snippet::from(
            Metadata::new(kind, transform.into(), another_method_span),
            r#"int Foo::another_method() { return 0; }"#.as_bytes(),
        ),
        Snippet::from(
            Metadata::new(kind, transform.into(), a_bare_function_span),
            r#"int a_bare_function( ){ return 0; }"#.as_bytes(),
        ),
    ];

    assert_eq!(extract, expected);
}

#[test]
fn function_in_class_signature_raw() {
    crate::tracing::setup();

    let kind = Kind::Signature;
    let transform = None;
    let member_method_span = Location::from(65..84);
    let another_method_span = Location::from(163..188);
    let a_bare_function_span = Location::from(264..310);

    let content = include_str_lf!("testdata/cpp_98/simple_class.cc");
    let opts = Options::new(Target::Function, kind, transform);

    let extract = cpp_98::Extractor::extract(&opts, &content).expect("extract snippets");
    let expected = vec![
        Snippet::from(
            Metadata::new(kind, transform.into(), member_method_span),
            member_method_span.extract_from(content.as_bytes()),
        ),
        Snippet::from(
            Metadata::new(kind, transform.into(), another_method_span),
            another_method_span.extract_from(content.as_bytes()),
        ),
        Snippet::from(
            Metadata::new(kind, transform.into(), a_bare_function_span),
            a_bare_function_span.extract_from(content.as_bytes()),
        ),
    ];

    assert_eq!(extract, expected);
}

#[test]
fn function_in_class_signature_code() {
    crate::tracing::setup();

    let kind = Kind::Signature;
    let transform = Some(Transform::Code);
    let member_method_span = Location::from(65..84);
    let another_method_span = Location::from(163..188);
    let a_bare_function_span = Location::from(264..310);

    let content = include_str_lf!("testdata/cpp_98/simple_class.cc");
    let opts = Options::new(Target::Function, kind, transform).disable_raw();

    let extract = cpp_98::Extractor::extract(&opts, &content).expect("extract snippets");
    let expected = vec![
        Snippet::from(
            Metadata::new(kind, transform.into(), member_method_span),
            member_method_span.extract_from(content.as_bytes()),
        ),
        Snippet::from(
            Metadata::new(kind, transform.into(), another_method_span),
            another_method_span.extract_from(content.as_bytes()),
        ),
        Snippet::from(
            Metadata::new(kind, transform.into(), a_bare_function_span),
            r#"int a_bare_function( )"#.as_bytes(),
        ),
    ];

    assert_eq!(extract, expected);
}
