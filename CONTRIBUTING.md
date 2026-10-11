# Contributing

```sh
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

- Keep code comments minimal and in English; documents are English (Japanese typesetting terms are also written in Japanese).
- The solver must stay pure and deterministic: same input, same output.
- Changes to scoring should come with a before/after look at `tools/measure/pipeline.mjs` output.

The site is `book/` (mdBook) plus the rustdoc output. `scripts/build-site.sh` builds it into `target/book` (it needs `mdbook`), and `scripts/serve-site.sh` previews it without browser caching. The chapters on formats and on the algorithm are `docs/SPEC.md` and `docs/ALGORITHM.md`.
