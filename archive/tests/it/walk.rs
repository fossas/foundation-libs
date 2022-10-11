use std::{collections::HashSet, path::PathBuf};

use archive::expand::walk;

use crate::testdata;

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
