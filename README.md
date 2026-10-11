# shinbun-layout

Automatic layout for Japanese newspaper pages (新聞の紙面) on the [shinbun-css](https://github.com/kamimen/shinbun-css) grid. Rust, no runtime dependencies besides `serde`.

Give it articles, photos and boxes; get back `{x, w, y, span}` placements — the same form shinbun-maker and shinbun-cli use.

## How it works

Text is never rendered here. Using it takes three steps:

1. `required_measurements(&problem)` — which sizes to measure.
2. Your host (a browser; see `tools/measure`) measures the minimum lines each item needs per column span (段).
3. `solve(&problem, &table, &options)` — pure, deterministic, returns placements, unplaced items with reasons, a score breakdown, warnings and alternatives.

`validate()` checks any layout, including forbidden patterns (シリモチ, 並列, エントツ, ハラキリ).

```sh
cargo run --example ascii          # draw a built-in front page
cargo run --example solve < in.json
node tools/measure/pipeline.mjs examples/maker-vertical.json out   # measure in Chrome, solve, write out/
```

JSON interface: `solve_json(problem, table, options)`.

## Status

Unreleased and unpublished. See [docs/ALGORITHM.md](docs/ALGORITHM.md) for the design, the measured assumption and known limits.

[日本語](README_ja.md)

## License

MIT
