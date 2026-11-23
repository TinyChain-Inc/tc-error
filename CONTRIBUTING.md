# Contributing to tc-error

Read the [crate invariants](AGENTS.md) and [API overview](README.md).
Contributions use the repository's [Apache-2.0 license](LICENSE).

## Standalone setup

In a fresh parent directory, clone this repository as `tc-error`, then run
`python3 ci/bootstrap.py` from the checkout. Repository-owned dependency pins in
`ci/dependencies.json` populate the required sibling paths. Bootstrap refuses
existing destinations and never resets developer checkouts. Install stable Rust
with rustfmt and Clippy, Python 3, and Git.

Commit the Cargo lockfile. When updating dependencies, publish compatible commits,
update pins and lockfile, and verify a fresh layout rather than relying on dirty
siblings. No parent-workspace scripts are required.

For private sibling repositories, configure the CI secret `DEPENDENCY_READ_TOKEN`
with read-only contents access to those repositories. CI uses GitHub CLI's Git
credential helper and falls back to its repository token for public dependencies.
Local bootstrap uses ordinary Git credentials. Fork pull requests do not receive
repository secrets; private dependency checks require an authorized run and must
not be silently skipped.

## Validation

```bash
cargo fmt -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --lib
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --lib --all-features
cargo test --locked --doc --all-features
```

CI also compiles optional adapters independently to catch missing feature
dependencies. It uses one Cargo build job, disabled incremental compilation and
debug symbols, finite timeouts, caching, and superseded-run cancellation. Numerical
conversions use the CPU backend configuration.

For iteration, select the affected target and case. Keep thorough conversion and
codec tests here; consumers test delegation and their own policy. Extend existing
cases for distinct failures rather than adding source-shape or prose-word tests.
README examples are included in crate doctests. Review logical spacing that
stable rustfmt does not enforce. Embedders own application panic policy.
