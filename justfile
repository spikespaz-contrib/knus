watch:
  watchexec -e rs,toml just test

test:
  cargo test --workspace

lint:
  cargo fmt --check
  cargo clippy --workspace --tests

cov:
  cargo +nightly llvm-cov --workspace --branch --open

ci-cov:
  cargo +nightly llvm-cov --workspace --branch --codecov --output-path codecov.json

check-wasm:
  cargo check --workspace --target wasm32-unknown-unknown

min-versions:
  cargo +nightly test --workspace -Z direct-minimal-versions
