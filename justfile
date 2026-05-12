set shell := ["bash", "-euo", "pipefail", "-c"]

fmt-check:
    cargo fmt --check

clippy:
    cargo clippy --all-targets --features wasm -- -D warnings

test-rust:
    cargo test
    cargo test --features wasm

check-wasm:
    cargo check --target wasm32-unknown-unknown --features wasm

install-js-deps:
    npm ci --prefix tests

build-npm:
    node scripts/build-npm-package.mjs

test-js:
    just install-js-deps
    node scripts/build-npm-package.mjs --link-dir tests/node_modules
    npm --prefix tests test

test:
    just test-rust
    just test-js

check:
    just fmt-check
    just clippy
    just check-wasm

check-release-version tag:
    #!/usr/bin/env bash
    set -euo pipefail
    crate_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
    tag_version="{{tag}}"
    tag_version="${tag_version#v}"
    if [ "$crate_version" != "$tag_version" ]; then
      echo "::error::Tag {{tag}} does not match Cargo.toml version ${crate_version}"
      exit 1
    fi

check-npm-package tag:
    node -e 'const pkg = require("./pkg/package.json"); const tagVersion = process.argv[1].replace(/^v/, ""); if (pkg.name !== "@necocen/piyoparse") throw new Error(`unexpected package name: ${pkg.name}`); if (pkg.version !== tagVersion) throw new Error(`npm package version ${pkg.version} does not match tag ${process.argv[1]}`);' "{{tag}}"
