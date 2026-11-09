#!/bin/sh
# CI gate: tests, lints, formatting. Integration tests create and detach temporary disk images with hdiutil.
set -eu
cd "$(dirname "$0")/.."
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
