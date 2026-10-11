# Algorithm

## Two phases

The solver never renders text. A browser measures, the solver decides.

1. `required_measurements(problem)` lists `(item, span)` pairs to measure.
2. The host returns, for each pair, the minimum number of lines (`w`) the item needs, or "cannot fit".
3. `solve(problem, table, options)` is pure and deterministic.

## Measured assumption

A text item's fit depends only on its shape `(span, w)`, not on where it sits, and is monotone in `w`.
Checked with `tools/measure/experiment.mjs` on Chrome 154, vertical writing, two article types, 10 columns × 60 lines:
0 disagreements between positions across 41 widths per span.
Not verified: horizontal writing, other browsers, other fonts, items with photos inside text.

## Search

- Items are ordered 押えて流す (fix first, then flow): Nameplate, Ad, Yoko, Photo, Box, Article (priority descending).
- Candidates per item: shapes (span × width) at anchor positions (edges and positions flush against placed items).
- Text frames: widths are the measured minimum plus small slack, aspect ratio capped (`max_aspect`).
  Non-text items (photo, ad, headline): a fixed ladder of widths.
- Beam search keeps the best `beam_width` partial layouts, with look-ahead (`leaves_room`) so later items are not starved; the greedy result is merged in, so beam is never worse.
- Occupancy is a `u64` bitmask per column.

## Score

Size, shape, position, coverage, hierarchy, forbidden patterns, unplaced — weights in `Weights`.
Position follows shinbun-css SPEC §10.8–10.9: dead corners (死角) for boxes and photos, diagonal for articles, headlines last.
Forbidden layouts (シリモチ, 並列, エントツ, ハラキリ) are warnings; overlap, out-of-bounds, fixed items moved and overflow are hard violations.

## Known limits

Short articles on a sparse page leave empty space inside frames; the page cannot be filled without more content.
No MILP / CP-SAT comparison yet. No WASM build yet.
