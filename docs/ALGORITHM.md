# Algorithm

## Two phases

The solver never renders text. A browser measures, the solver decides.

1. `required_measurements(problem)` lists `(item, span)` pairs to measure.
2. The host returns, for each pair, the minimum number of lines (`w`) the item needs, or "cannot fit".
3. `solve(problem, table, options)` is pure and deterministic.

## Measured assumption

A text item's fit depends only on its shape `(span, w)`, not on where it sits, and is monotone in `w`.
Checked with `tools/measure/experiment.mjs` (`BROWSER=chromium|firefox|webkit`, `WRITING=vertical|horizontal`), two article types, 10 columns × 60 lines, 41 widths per span:

| browser | vertical | horizontal |
|---|---|---|
| Chrome 154 | 0 disagreements | 0 disagreements |
| WebKit (Playwright 2370) | 0 disagreements | 0 disagreements |
| Firefox | not run: the Playwright build does not start on the development machine | not run |

No case was non-monotone (fits at `w` but not at a larger `w`).
Not verified: Firefox, other fonts, items with photos inside text.

Two details of the measuring host matter. Articles overflow at the end of the block direction, but a box (囲み) overflows along the line axis, so boxes are checked on the other side. And the font size (`--sb-fs`) must be passed to the measuring page, otherwise every item measures as one line.

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

## Solvers

- `Constructive`: the greedy method.
- `Beam`: beam search, never worse than greedy.
- `Refine` (the default of `solve`): the beam result, then ruin-and-recreate under simulated annealing. Each step removes one to three neighbouring items together with the ones left unplaced, and puts them back by trying every combination of their best placements (up to four items), or leaving some out if that scores higher. With `refine_iterations: 0` it is the plain beam.

## Fitting what is left out

When the annealing leaves an item unplaced, `separate` tries to fit everything at once: every item may overlap for a while, one conflicting item at a time moves to the shape and position where it overlaps least (all of them are scored together with prefix sums of the cell weights), and cells that stay in conflict get heavier. This is the separation step of the nesting heuristic `sparrow` (https://github.com/JeroenGar/sparrow), on a grid. On a 10 × 50 page with 12 items it placed the one article the annealing could not, in 0.02 s.

## The exact model

The problem is a weighted set packing: one 0-1 variable for every (item, shape, position), each item takes at most one, each cell is covered at most once. The terms that involve two items (hierarchy, forbidden patterns) are not linear, so the model holds only the single-item terms ("separable"). `crates/shinbun-layout-exact` solves it with HiGHS and is used as a yardstick, not shipped.

On 20 random pages of 6 columns × 24 lines with 5 movable items (`cargo run --release -p shinbun-layout-exact --bin gap -- 20 5`), gap to the exact optimum of the separable score:

| solver | mean gap | median | worst |
|---|---|---|---|
| Beam | 13.9% | 13.3% | 38.4% |
| Refine (default) | 4.7% | 2.4% | 23.6% |

Exact takes 0.1 to 12 s per page; Refine takes at most 0.9 s (native, one core). Refine stays more than 10% below the optimum on 3 of the 20 pages. The two-item terms are not in this comparison, so the real total score can differ; on these pages Refine's real total was above the exact layout's in 5 of 20 cases (the beam's never was), because the exact model ignores them.

## Browsers

Frames are exact: a column holds `chars` characters, a frame `w` lines, and a rounding error of 0.03 px in some browsers is enough to wrap a line early or drop the last line (seen in a desktop Chrome tab, not in headless Chrome). The CLI therefore gives text frames 0.1 em of room along both axes (`vendor/paper.css`), measures with a 0.25 px tolerance, and offers `--margin` for fonts that run wider.

## Known limits

Short articles on a sparse page leave empty space inside frames; the page cannot be filled without more content.
Refine is a heuristic: on tight pages it can leave an item out that the optimum places.
The WebAssembly build gave the same result as the native one on the problems it was compared on.

## Code

One library crate. `model` (the types), `validate`, `candidates` (shapes and anchor positions), `scoring`, and `solver/` (`beam`, `refine`, `separate`) depend on each other in that order and on nothing but `serde`; `json` is the text interface. `shinbun-layout-exact` (not published) solves the same problem with HiGHS to measure the gap to the optimum, through `shinbun_layout::__internal`.
