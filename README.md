# foundation-libs

Foundational libraries and helper binaries for cross-language support, written in Rust.
<sup>_[(TOC?)](https://github.blog/changelog/2021-04-13-table-of-contents-support-in-markdown-files/)_</sup>

> [!IMPORTANT]
> This repository is archived, but most packages within it are still considered actively maintained.
>
> As we make changes to these packages we'll extract them into their own repositories
> (and shared libraries into their own repositories).
>
> This extraction is planned to be performed lazily (in the programming sense),
> meaning that we'll extract the package the first time we need to make changes.
> If a repository below doesn't yet exist, or is empty, this repository is the canonical latest version.
>
> You can find the new repositories below:
> - If it was a binary, it'll be in a new repository named after the binary.
>   - `tracer`: planned to be folded into `https://github.com/fossas/diagnose`.
> - If it was a library, it'll be in a new repository named after the library, usually prepended with `lib-` or suffixed by the language (`-rs`).
>   The intention is that if the library is meant to be canonical (so it's used across languages, via FFI) it gets the `lib-` prefix.
>   If it's language specific, it gets the `-rs` suffix.
>   - `srclib`: `https://github.com/fossas/srclib-rs`
>   - `traceconf`: `https://github.com/fossas/traceconf-rs`
>   - `snippets`: `https://github.com/fossas/lib-snippets`
>   - `archive`: `https://github.com/fossas/lib-archive`
>   - `berkeleydb`: `https://github.com/fossas/lib-berkeleydb`
>   - `fingerprint`: Folding into `vsi`.
>   - `vsi`: planned to be exposed as a library only at `https://github.com/fossas/lib-vsi`.

## finding your way around

Cargo [crates](terminology) are organized depending on the kind of crate.
- For library crates, the entrypoint is `src/lib.rs`.
- For binary crates, the entrypoint is `src/main.rs`, and the `main()` function within.
- Some crates are both; usually in these cases the binary crate depends on the library crate.
  This is commonly done to separate things like "running the program in a shell" from "testing the program".

Usually if you're interested in how a library or binary functions, you want to navigate
to that entrypoint file and use an LSP enabled code editor to jump to definition on the things it calls.

### documentation

Many software projects make extensive use of README docs (like this one!) to document the code.
Rust projects are a little different: the built in Rust documentation is extremely good, and is a superset of
Markdown: Rust docs can provide all the same formatting, but with added semantic context and tested examples
from the project itself.

Given this, it's most common to see relatively sparse markdown documentation, and instead use Rust
doc comments for your documentation needs. Markdown files like this one usually lay out very broad concepts
(again, like this one) or discuss something that's not within the scope of the Rust code (for example,
how to set up a local testing database).

There's two a few things to be aware of, and you'll be able to use doc comments like a pro:
1. Comments denoted with `///` attach documentation to the symbol immediately following the comment.
2. Comments denoted with `//!` attach documentation to the symbol in which the documentation is nested.
3. Comments contain seemingly broken links (e.g., ``[`Symbol`]`` or ``[`super::Symbol`]``);
   these mean "link to the documentation for the named symbol in the current Rust scope".
   Depending on your text editor clicking these links may not work if the destination isn't published
   on `crates.io`, but you can always perform this lookup yourself.

## onboarding

The libraries in this repo attempt to track the latest version of the Rust compiler and associated tooling at all times.
Similarly, they attempt to track the latest edition of the Rust language.

Development is performed on `main`. TBD how releases are performed, but will probably be pushed to cargo.
Libraries in this repo should follow [semver](https://semver.org/):
- MAJOR version when you make incompatible API changes,
- MINOR version when you add functionality in a backwards compatible manner, and
- PATCH version when you make backwards compatible bug fixes.
  Additional labels for pre-release and build metadata are available as extensions to the MAJOR.MINOR.PATCH format. 

### terminology

#### tooling

- `rustc`: the actual rust compiler.
- `cargo`: manages projects by installing dependencies, testing, building, and linking.
  The [Cargo Book](https://doc.rust-lang.org/cargo/index.html)
  is an excellent resource for learning how it works and how to use it.
- `clippy`: the official linter for rust projects.
- `rustfmt`: the official code formatter for rust projects.

#### concepts

- `crate`: a compiled code unit; a "library" or a "binary".

### setting up your development environment

- Install Rust: https://www.rust-lang.org/tools/install
- (Recommended) Install [`cargo edit`](https://lib.rs/crates/cargo-edit): `cargo install cargo-edit`
  - This makes it simpler to edit your cargo dependencies from the CLI.
- (Recommended) Install [`cargo nextest`](https://nexte.st/book/pre-built-binaries.html): https://nexte.st/
  - This provides much faster and nicer test running, along with several other benefits.

We recommend Visual Studio Code with the `rust-analyzer` extension,
however a close runner up is CLion/IntelliJ with the `IntelliJ Rust` plugin.

### code organization

This repo is managed as a `cargo` [workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html).

### style guide

Make your code look like the code around it. Consistency is the name of the game.

You should submit changes to this doc if you think you can improve it,
or if a case should be covered by this doc, but currently is not.

Use `rustfmt` for formatting.
Our CI setup enforces that all changes pass a `rustfmt` run with no differences.

Our CI systems ensure that all patches pass `clippy` checks.

Comments should describe the "why", type signatures should describe the "what", and the code should describe the "how".

We use the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/about.html)
during code review; if you want to get ahead of the curve check it out!
