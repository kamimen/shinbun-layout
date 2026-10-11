#!/bin/sh
# Builds the documentation site into target/book: the mdBook guide, with the rustdoc output under api/.
# Needs mdbook. Old rustdoc output is removed first, so a crate that no longer exists cannot linger in the list.
set -eu

cargo clean --doc
mdbook build book
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --exclude shinbun-layout-exact
mkdir -p target/book/api
cp -R target/doc/. target/book/api/
