# Changelog

## 0.1.0

- Two-phase API: `required_measurements` → measurement table → `solve`; `validate` for any layout.
- WebAssembly build (`scripts/build-wasm.sh`): `requiredMeasurements` and `solve`, identical to the native result.
- One library crate (`shinbun-layout`, with the `shinbun-layout` command) and `shinbun-layout-wasm`; the public API is built from constructors (`Item::new`, `Paper::new`, `SolveOptions` setters), and the JSON calls live in `json` with a typed `json::Error`.
- Solvers: greedy, beam search, and `Refine` (ruin-and-recreate with simulated annealing, the default).
- `paper.column_em` and `paper.gap_em`: the real length of a column, so pictures keep their ratio on horizontal pages too.
- `separate_moves`: after the annealing, items that are still left out are fitted by overlap minimisation (guided local search, after sparrow).
- Photos keep the ratio of their picture (`ratio`, `caption`): only frames close to it are candidates.
- Forbidden patterns as warnings (シリモチ, 並列, エントツ, ハラキリ), with the items involved.
- Reference measurement host in `tools/measure` (Playwright; Chrome, WebKit, and Firefox where it starts), and `shinbun-layout-exact`, a HiGHS model used to measure the gap to the optimum.
