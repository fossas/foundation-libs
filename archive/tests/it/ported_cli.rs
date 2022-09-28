//! Tests ported from https://github.com/fossas/fossa-cli.

use archive::{expand, Options, Source};
use log::debug;

use crate::testdata;

/// https://github.com/fossas/fossa-cli/blob/219bdc6f38d401df2bdb7991114c54083a75f56b/test/Discovery/ArchiveSpec.hs#L30-L45
#[test]
fn extract_simple_zip() {
    pretty_env_logger::init();

    let target = testdata::target("testdata/simplezip");
    let opts = Options::default();

    let result = expand::all(target.clone(), opts).unwrap();
    debug!("extracted: {:?}", result.locations());

    let destination = result
        .locations()
        .get_by_left(&Source::from(target.root().join("simple.zip")))
        .unwrap();

    testdata::assert_content(
        destination,
        vec![
            ("simple/a.txt", b"6b5effe3-215a-49ec-9286-f0702f7eb529"),
            ("simple/b.txt", b"8dea86e4-4365-4711-872b-6f652b02c8d9"),
        ],
    );
}

/// https://github.com/fossas/fossa-cli/blob/219bdc6f38d401df2bdb7991114c54083a75f56b/test/Discovery/ArchiveSpec.hs#L47-L62
#[test]
fn extract_simple_tar() {
    pretty_env_logger::init();

    let target = testdata::target("testdata/simple.tar");
    let opts = Options::default();

    let result = expand::all(target.clone(), opts).unwrap();
    debug!("extracted: {:?}", result.locations());

    let destination = result
        .locations()
        .get_by_left(&Source::from(target.root().to_owned()))
        .unwrap();

    testdata::assert_content(
        destination,
        vec![
            ("simple/a.txt", b"6b5effe3-215a-49ec-9286-f0702f7eb529"),
            ("simple/b.txt", b"8dea86e4-4365-4711-872b-6f652b02c8d9"),
        ],
    );
}

/// https://github.com/fossas/fossa-cli/blob/219bdc6f38d401df2bdb7991114c54083a75f56b/test/Discovery/ArchiveSpec.hs#L64-L79
#[test]
fn extract_simple_tar_gz() {
    pretty_env_logger::init();

    let target = testdata::target("testdata/simple.tar.gz");
    let opts = Options::default();

    let result = expand::all(target.clone(), opts).unwrap();
    debug!("extracted: {:?}", result.locations());

    let destination = result
        .locations()
        .get_by_left(&Source::from(target.root().to_owned()))
        .unwrap();

    testdata::assert_content(
        destination,
        vec![
            ("simple/a.txt", b"6b5effe3-215a-49ec-9286-f0702f7eb529"),
            ("simple/b.txt", b"8dea86e4-4365-4711-872b-6f652b02c8d9"),
        ],
    );
}
