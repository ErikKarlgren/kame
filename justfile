default:
    @just check

check: fmt
    cargo check --all-targets --all-features

clippy: fmt
    cargo clippy --all-targets --all-features --fix --allow-dirty

test: fmt clippy
    cargo test

fmt:
    cargo fmt

fmt-check:
    cargo fmt -- --check

ci: fmt-check clippy test

build:
    cargo build --all-targets --all-features

run *ARGS:
    cargo run -- {{ARGS}}
