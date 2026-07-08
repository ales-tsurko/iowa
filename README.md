# Gobbledygook

**Gobbledygook** (or `gg`) is an [**Io**](https://iolanguage.org)-inspired
compiled language experiment.

## Requirements

Install the usual Rust toolchain first.

Extra development tools:

```sh
rustup toolchain install nightly --component miri --component rust-src --component rustfmt
cargo install cargo-binstall
cargo binstall cargo-llvm-cov cargo-crap similarity-rs cargo-machete --no-confirm
```

Nightly Rust is required for repository formatting. `cargo-llvm-cov` generates
LCOV coverage data for `cargo-crap`. `cargo-machete` checks unused dependencies,
`similarity-rs` checks duplicated Rust code, and Miri checks interpreter-level
undefined behavior in Rust tests.

## Commands

Use Makefile targets for project commands.

```sh
make check       # cargo check for workspace crates
make test-all    # run all tests
make lint        # clippy and nightly rustfmt check
make fmt         # format with nightly rustfmt
make code-health # similarity, unused deps, coverage/CRAP, and Miri
```

`make code-health` writes:

```text
target/gg-lcov.info
target/gg-crap.md
```

No direct cargo commands are used in CI; CI calls the same Makefile targets.
