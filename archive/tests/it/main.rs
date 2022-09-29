//! Integration tests.
//!
//! Along with new tests, these integration tests are a mixture of the tests in:
//! - `fossa-cli` (Haskell)
//!
//! The idea is that since this library is meant to be able to replace _all of these_,
//! it should pass all of their test suites.
//! (This uses wording to imply multiple other projects, but those aren't as high priority;
//! still this is something that's going to be considered in the future).
//!
//! The tests that are ported from another repo are in `ported_<repo>` sibling modules.
//! New tests are in this or other modules.
//!
//! Tests in this module set up env_logger; use `RUST_LOG=debug` to see its output.

mod local;
mod ported_cli;
mod testdata;
