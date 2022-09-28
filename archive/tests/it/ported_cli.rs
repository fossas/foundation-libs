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

/// https://github.com/fossas/fossa-cli/blob/219bdc6f38d401df2bdb7991114c54083a75f56b/test/Discovery/ArchiveSpec.hs#L81-L96
#[test]
fn extract_simple_tar_xz() {
    pretty_env_logger::init();

    let target = testdata::target("testdata/simple.tar.xz");
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

/// https://github.com/fossas/fossa-cli/blob/219bdc6f38d401df2bdb7991114c54083a75f56b/test/Discovery/ArchiveSpec.hs#L98-L113
#[test]
fn extract_simple_tar_bz2() {
    pretty_env_logger::init();

    let target = testdata::target("testdata/simple.tar.bz2");
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

/// https://github.com/fossas/fossa-cli/blob/219bdc6f38d401df2bdb7991114c54083a75f56b/test/Discovery/ArchiveSpec.hs#L115-L121
#[test]
fn extract_el7_rpm() {
    pretty_env_logger::init();

    let target = testdata::target("testdata/curl-7.29.0-59.el7.x86_64.rpm");
    let opts = Options::default();

    let result = expand::all(target.clone(), opts).unwrap();
    debug!("extracted: {:?}", result.locations());

    let destination = result
        .locations()
        .get_by_left(&Source::from(target.root().to_owned()))
        .unwrap();

    testdata::assert_hashed_content(
        destination,
        vec![
            (
                "usr/bin/curl",
                "47dbba060d72829769ef29ea3675b82e6df47b12b54dee7a09f4801926b74dc9",
            ),
            (
                "usr/share/man/man1/curl.1.gz",
                "8b42fd5707727da8ef67636370c20322101f4c5ebe99d999aef185e9acb37ee6",
            ),
            (
                "usr/share/doc/curl-7.29.0/TheArtOfHttpScripting",
                "e8726c9baf3cadda2845ecff7e6580db784e84283f698a69293d79156875425c",
            ),
            (
                "usr/share/doc/curl-7.29.0/FAQ",
                "f43a2c6f882ccbe3cbbb56ce1f472642644e48adb32d7d73fde31685df2b1785",
            ),
            (
                "usr/share/doc/curl-7.29.0/CHANGES",
                "23ca1d732e2523a2b591fd196ec5ddd7daa48a1b4bf59886560878aaea468b99",
            ),
            (
                "usr/share/doc/curl-7.29.0/RESOURCES",
                "985a2d39c877b847da64ee73d0e5afa0431af546fe8ffe90cd1882675a87b217",
            ),
            (
                "usr/share/doc/curl-7.29.0/FEATURES",
                "a78c0c8a3952e9a2bf3204ecce5ba3fbd0e01f127baf8175cbb8db3865e6b9f0",
            ),
            (
                "usr/share/doc/curl-7.29.0/BUGS",
                "24bbe914ac0937906c745115fa9d14b5e7e5cec0332998683d84b76a791d57bb",
            ),
            (
                "usr/share/doc/curl-7.29.0/README",
                "5540c522b6d62887dca72ed06345b88b1603a01072418b8fc93a7798b1560359",
            ),
            (
                "usr/share/doc/curl-7.29.0/TODO",
                "d47375aff721538403f09e5d1f77583efb3634f5b0538e33cbf8bfaf4584389b",
            ),
            (
                "usr/share/doc/curl-7.29.0/COPYING",
                "85a861a77b1c1dd6cbf4b5e4edca2def74af30de7b40c0b05131c32dd66b1081",
            ),
            (
                "usr/share/doc/curl-7.29.0/MANUAL",
                "6c3f52d84241d76d371c20be52e22bf4f7fba3fd554f35c23323b5236bc26c32",
            ),
        ],
    );
}
