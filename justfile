watch:
  watchexec -e rs,toml just test

test:
  cargo test --workspace

# FIXME: Get rid of these -A flags
lint:
  cargo fmt --check
  cargo clippy --workspace --tests -- -W clippy::nursery -W clippy::pedantic -W clippy::cargo -A clippy::missing_errors_doc -A clippy::cargo_common_metadata -A clippy::multiple_crate_versions

cov:
  cargo +nightly llvm-cov --workspace --branch --open

ci-cov:
  cargo +nightly llvm-cov --workspace --branch --codecov --output-path codecov.json

check-wasm:
  cargo check --workspace --target wasm32-unknown-unknown

min-versions:
  cargo +nightly test --workspace -Z direct-minimal-versions
