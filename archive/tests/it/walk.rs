use std::{collections::HashSet, path::PathBuf};

use archive::{expand::walk, Options, Recursion};

use crate::testdata::{self, assert_walked_hashed_content};

#[test]
fn walks_archive() {
    let target = testdata::target("testdata/simplezip");
    let walker = walk(target, Default::default());

    let expected = vec![
        "simple.zip",
        "simple.zip!_fossa.virtual_!/simple/a.txt",
        "simple.zip!_fossa.virtual_!/simple/b.txt",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect::<HashSet<_>>();

    let got = walker
        .map(|e| e.map(|e| e.path().to_owned()))
        .collect::<Result<HashSet<_>, _>>()
        .expect("must have expanded");

    assert_eq!(got, expected);
}

#[test]
fn extract_simple_zip() {
    let target = testdata::target("testdata/simplezip");
    let walker = walk(target, Default::default());

    assert_walked_hashed_content(
        walker,
        vec![
            (
                "simple.zip",
                "65edda9e1933aa8cff1d5aeec70a8ddbd43f971454b982f101aa9beff0b72901",
            ),
            (
                "simple.zip!_fossa.virtual_!/simple/a.txt",
                "a1521f679d5583c4bac29209c655c04a6cadb68a364d448d7b43224aeffd82ce",
            ),
            (
                "simple.zip!_fossa.virtual_!/simple/b.txt",
                "367a5b6e6b67fa0c2d00dee7c91eb3f0d85a93e537335abbed7908c9f87738c8",
            ),
        ],
    );
}

#[test]
fn extract_simple_zip_custom_suffix() {
    let target = testdata::target("testdata/simplezip");
    let options = Options::builder().archive_postfix("~postfix").build();
    let walker = walk(target, options);

    assert_walked_hashed_content(
        walker,
        vec![
            (
                "simple.zip",
                "65edda9e1933aa8cff1d5aeec70a8ddbd43f971454b982f101aa9beff0b72901",
            ),
            (
                "simple.zip~postfix/simple/a.txt",
                "a1521f679d5583c4bac29209c655c04a6cadb68a364d448d7b43224aeffd82ce",
            ),
            (
                "simple.zip~postfix/simple/b.txt",
                "367a5b6e6b67fa0c2d00dee7c91eb3f0d85a93e537335abbed7908c9f87738c8",
            ),
        ],
    );
}

#[test]
fn extract_nested_archives() {
    let target = testdata::target("testdata/nested");
    let options = Options::builder().archive_postfix("").build();
    let walker = walk(target, options);

    let expected = vec![
        ("nested.zip", "efa3a1b5d22aaaa0593a47434752ec405c1c40aad20252a572fb90f95507316e"),
        ("nested.zip/inner.zip", "4b88bb511827953f6479fe50bf1e36d1a4428254e1bd380113956249bf0b6c33"),
        ("nested.zip/inner.zip/simple.tar.bz2", "10cf12de77d827e04ac414c7774ec2488f2f0102e1206bb7624b7fc6aeea861d"),
        ("nested.zip/inner.zip/simple.tar.bz2/simple/a.txt", "a1521f679d5583c4bac29209c655c04a6cadb68a364d448d7b43224aeffd82ce"),
        ("nested.zip/inner.zip/simple.tar.bz2/simple/b.txt", "367a5b6e6b67fa0c2d00dee7c91eb3f0d85a93e537335abbed7908c9f87738c8"),
        ("nested.zip/inner.zip/curl-7.78.0-3.fc35.x86_64.rpm", "69e8b4c6ea1f0f291ef283d1e076de7ba3c9388b3558d0ab492fa2e675ab4813"),
        (
            "nested.zip/inner.zip/curl-7.78.0-3.fc35.x86_64.rpm/usr/bin/curl",
            "f44294f0cb31bdbddd7dde93702a98dc9de60a0c0207f0e7ee6df7631a9ae641",
        ),
        (
            "nested.zip/inner.zip/curl-7.78.0-3.fc35.x86_64.rpm/usr/lib/.build-id/b3/1694338b7ba8cedd532408473dbac8ebe509c5",
            "f44294f0cb31bdbddd7dde93702a98dc9de60a0c0207f0e7ee6df7631a9ae641",
        ),
        (
            "nested.zip/inner.zip/curl-7.78.0-3.fc35.x86_64.rpm/usr/share/man/man1/curl.1.gz",
            "e3ab38e59cda834a11cee0ae4659dc6d609d8ed1f3e3c80dcd0d28cb56908d4c",
        ),
        (
            "nested.zip/inner.zip/curl-7.78.0-3.fc35.x86_64.rpm/usr/share/doc/curl/BUGS.md",
            "c5fc32214134097232490fa9e0d3cd1f299b04f5e550c2bfc8ff081ff29f0836",
        ),
        (
            "nested.zip/inner.zip/curl-7.78.0-3.fc35.x86_64.rpm/usr/share/doc/curl/FAQ",
            "d00231e857aa821f9ca1519681463fafea852bb437c9b0ca49ab341bdee04b55",
        ),
        (
            "nested.zip/inner.zip/curl-7.78.0-3.fc35.x86_64.rpm/usr/share/doc/curl/CHANGES",
            "0ab7f82274290a06b6a5d78dab3097c8a589d1f77325e64be32599c522b7dd96",
        ),
        (
            "nested.zip/inner.zip/curl-7.78.0-3.fc35.x86_64.rpm/usr/share/doc/curl/TheArtOfHttpScripting.md",
            "600d0796844ccf177b452d2f3abed65eb1f454c2685381d479f76c2dafc83789",
        ),
        (
            "nested.zip/inner.zip/curl-7.78.0-3.fc35.x86_64.rpm/usr/share/doc/curl/README",
            "ce118b51897f4452dcbe7d2042f05222fd2a8c0362ca177b3cd6c6fb3a335548",
        ),
        (
            "nested.zip/inner.zip/curl-7.78.0-3.fc35.x86_64.rpm/usr/share/doc/curl/TODO",
            "06e052269d2ec3f08b65e257d7b65609e3b678c0e2143c4410ca667c097baa2b",
        ),
        (
            "nested.zip/inner.zip/curl-7.78.0-3.fc35.x86_64.rpm/usr/share/doc/curl/FEATURES.md",
            "ceecb9363eb82c80a7096064d01f89abf2149c3e66b92674966f2707dd10b83a",
        ),
        (
            "nested.zip/inner.zip/curl-7.78.0-3.fc35.x86_64.rpm/usr/share/zsh/site-functions/_curl",
            "ee8fe9041235e96a89c66ab3accc6f6bb6a9cc473566031c51fb0553d830f258",
        ),
        ("nested.zip/simple.tar.xz", "1ad70c7e7dcf0ccb420ea617ec94e59747f5bc8c168e6dd506fb69e223c10615"),
        ("nested.zip/simple.tar.xz/simple/a.txt", "a1521f679d5583c4bac29209c655c04a6cadb68a364d448d7b43224aeffd82ce"),
        ("nested.zip/simple.tar.xz/simple/b.txt", "367a5b6e6b67fa0c2d00dee7c91eb3f0d85a93e537335abbed7908c9f87738c8"),
        ("nested.zip/simplezip/simple.zip", "65edda9e1933aa8cff1d5aeec70a8ddbd43f971454b982f101aa9beff0b72901"),
        ("nested.zip/simplezip/simple.zip/simple/a.txt", "a1521f679d5583c4bac29209c655c04a6cadb68a364d448d7b43224aeffd82ce"),
        ("nested.zip/simplezip/simple.zip/simple/b.txt", "367a5b6e6b67fa0c2d00dee7c91eb3f0d85a93e537335abbed7908c9f87738c8"),
    ];

    assert_walked_hashed_content(walker, expected);
}

#[test]
fn extract_nested_archives_depth() {
    let target = testdata::target("testdata/nested");
    let options = Options::builder()
        .archive_postfix("")
        .recursion(Recursion::Enabled { depth: 1 })
        .build();
    let walker = walk(target, options);

    let expected = vec![
        (
            "nested.zip",
            "efa3a1b5d22aaaa0593a47434752ec405c1c40aad20252a572fb90f95507316e",
        ),
        (
            "nested.zip/inner.zip",
            "4b88bb511827953f6479fe50bf1e36d1a4428254e1bd380113956249bf0b6c33",
        ),
        (
            "nested.zip/simple.tar.xz",
            "1ad70c7e7dcf0ccb420ea617ec94e59747f5bc8c168e6dd506fb69e223c10615",
        ),
        (
            "nested.zip/simplezip/simple.zip",
            "65edda9e1933aa8cff1d5aeec70a8ddbd43f971454b982f101aa9beff0b72901",
        ),
    ];

    assert_walked_hashed_content(walker, expected);
}

#[test]
fn extract_nested_archives_no_recursion() {
    let target = testdata::target("testdata/nested");
    let options = Options::builder()
        .archive_postfix("")
        .recursion(Recursion::Disabled)
        .build();
    let walker = walk(target, options);

    let expected = vec![(
        "nested.zip",
        "efa3a1b5d22aaaa0593a47434752ec405c1c40aad20252a572fb90f95507316e",
    )];

    assert_walked_hashed_content(walker, expected);
}
