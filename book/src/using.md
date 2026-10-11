# Using it

From Rust:

```rust
let queries = shinbun_layout::required_measurements(&problem);
// measure `queries` in a browser and fill a MeasurementTable
let solution = shinbun_layout::solve(&problem, &table, &SolveOptions::default())?;
```

From JavaScript, through WebAssembly (`scripts/build-wasm.sh` writes `pkg/web` and `pkg/node`):

```js
import init, { requiredMeasurements, solve } from "./pkg/web/shinbun_layout_wasm.js";
await init();
const queries = JSON.parse(requiredMeasurements(problemJson));
const solution = JSON.parse(solve(problemJson, tableJson, ""));   // errors are thrown as strings
```

From the command line, or any language, with JSON files:

```sh
shinbun-layout measurements problem.json
shinbun-layout solve problem.json table.json
shinbun-layout validate problem.json table.json layout.json
```

`tools/measure` is a reference measuring host (Playwright and Chrome). `cargo run --example ascii` draws a built-in front page as text.

## Build

```sh
git clone https://github.com/kamimen/shinbun-layout
cd shinbun-layout
cargo build --release          # target/release/shinbun-layout
cargo test
scripts/build-wasm.sh          # needs the wasm32 target and wasm-bindgen-cli (the script says which version)
```
