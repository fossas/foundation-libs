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

/// https://github.com/fossas/fossa-cli/blob/219bdc6f38d401df2bdb7991114c54083a75f56b/test/Discovery/ArchiveSpec.hs#L123-L129
#[test]
fn extract_fc35_rpm() {
    pretty_env_logger::init();

    let target = testdata::target("testdata/curl-7.78.0-3.fc35.x86_64.rpm");
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
                "f44294f0cb31bdbddd7dde93702a98dc9de60a0c0207f0e7ee6df7631a9ae641",
            ),
            (
                // This is actually different than the one in CLI: https://github.com/fossas/fossa-cli/blob/219bdc6f38d401df2bdb7991114c54083a75f56b/test/Discovery/ArchiveSpec.hs#L179
                // But as far as I can tell, this is accurate:
                // ```
                // ; tar -xf testdata/curl-7.78.0-3.fc35.x86_64.rpm -C fc45
                // ; shasum /var/folders/q7/3nvvpy0d6js28m8lypw3tcx80000gn/T/.tmp4C7dnS/usr/lib/.build-id/b3/1694338b7ba8cedd532408473dbac8ebe509c5
                // 7611d80ac0d2bfa314bcb60092c6e70050850ac9  /var/folders/q7/3nvvpy0d6js28m8lypw3tcx80000gn/T/.tmp4C7dnS/usr/lib/.build-id/b3/1694338b7ba8cedd532408473dbac8ebe509c5
                // ; shasum /var/folders/q7/3nvvpy0d6js28m8lypw3tcx80000gn/T/.tmp4C7dnS/usr/bin/curl
                // 7611d80ac0d2bfa314bcb60092c6e70050850ac9  /var/folders/q7/3nvvpy0d6js28m8lypw3tcx80000gn/T/.tmp4C7dnS/usr/bin/curl
                // ```
                "usr/lib/.build-id/b3/1694338b7ba8cedd532408473dbac8ebe509c5",
                "f44294f0cb31bdbddd7dde93702a98dc9de60a0c0207f0e7ee6df7631a9ae641",
            ),
            (
                "usr/share/man/man1/curl.1.gz",
                "e3ab38e59cda834a11cee0ae4659dc6d609d8ed1f3e3c80dcd0d28cb56908d4c",
            ),
            (
                "usr/share/doc/curl/BUGS.md",
                "c5fc32214134097232490fa9e0d3cd1f299b04f5e550c2bfc8ff081ff29f0836",
            ),
            (
                "usr/share/doc/curl/FAQ",
                "d00231e857aa821f9ca1519681463fafea852bb437c9b0ca49ab341bdee04b55",
            ),
            (
                "usr/share/doc/curl/CHANGES",
                "0ab7f82274290a06b6a5d78dab3097c8a589d1f77325e64be32599c522b7dd96",
            ),
            (
                "usr/share/doc/curl/TheArtOfHttpScripting.md",
                "600d0796844ccf177b452d2f3abed65eb1f454c2685381d479f76c2dafc83789",
            ),
            (
                "usr/share/doc/curl/README",
                "ce118b51897f4452dcbe7d2042f05222fd2a8c0362ca177b3cd6c6fb3a335548",
            ),
            (
                "usr/share/doc/curl/TODO",
                "06e052269d2ec3f08b65e257d7b65609e3b678c0e2143c4410ca667c097baa2b",
            ),
            (
                "usr/share/doc/curl/FEATURES.md",
                "ceecb9363eb82c80a7096064d01f89abf2149c3e66b92674966f2707dd10b83a",
            ),
            (
                "usr/share/zsh/site-functions/_curl",
                "ee8fe9041235e96a89c66ab3accc6f6bb6a9cc473566031c51fb0553d830f258",
            ),
        ],
    );
}
