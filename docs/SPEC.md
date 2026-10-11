# Specification

All JSON uses the names of shinbun-css and shinbun-maker.

## Grid

A paper has `dan` columns (段, at most 15) of `chars` characters, and `lines` lines along the line axis (at most 60).
`paper` may also give `column_em` (the length of one column in character sizes, default `chars`) and `gap_em` (the space between columns, default 1). A frame that spans `span` columns is `span × column_em + (span − 1) × gap_em` long. Horizontal pages are as wide as the paper, so their columns are not `chars` wide; the host measures them.
A placement `{x, w, y, span}` is 1-based: `x`/`w` run along the line axis (from the right in vertical writing, 縦組み; from the left in horizontal writing, 横組み), `y`/`span` run along the columns from the top.

## Problem

```json
{
  "paper": { "writing": "vertical", "dan": 10, "lines": 50, "chars": 10 },
  "items": [
    { "id": "n", "role": "nameplate", "fixed": { "x": 1, "w": 8, "y": 1, "span": 3 } },
    { "id": "a", "role": "article", "priority": 10, "chars": 212, "headline": "l", "lead": true },
    { "id": "p", "role": "photo", "priority": 6 }
  ]
}
```

| field | meaning |
|---|---|
| `role` | `nameplate`, `ad`, `yoko` (horizontal headline), `photo`, `box`, `article` |
| `priority` | 1 (minor) to 10 (lead); default 5 |
| `chars` | text length, used to estimate a size |
| `headline` | `xl`, `l`, `m`, `s` |
| `lead` | the lead story (トップ記事) |
| `ratio` | photos: width over height of the picture; only frames close to it are tried (within 12%) |
| `caption` | photos: the frame holds a caption, which takes one line |
| `min_span`, `max_span` | limits on the number of columns |
| `fixed` | the item keeps this placement |

Malformed input (duplicate ids, priority out of range, fixed items outside the paper or overlapping, grid too large) is an error. A well-formed problem that cannot be satisfied is not: items that do not fit come back in `unplaced`.

## Measurements

`required_measurements(problem)` lists `{id, span}` for every movable item and every allowed span.
The host answers with a table of `{id, span, lines}` where `lines` is the least number of lines the item needs at that span, or `null` if it cannot fit at any width. Items without text (photo, ad, headline) report `1`.
The solver assumes the fit depends on the shape only, and is monotone in the width; see [ALGORITHM.md](ALGORITHM.md) for how this was checked.

## Solution

```json
{
  "layout": { "placements": [ { "id": "a", "x": 14, "w": 17, "y": 4, "span": 4 } ] },
  "unplaced": [ { "id": "b", "reason": "no_space" } ],
  "score": { "size": 0, "shape": 0, "position": 0, "coverage": 0, "hierarchy": 0, "forbidden": 0, "unplaced": 0, "total": 0 },
  "warnings": [ { "type": "forbidden", "pattern": "harakiri", "ids": ["a", "b"] } ],
  "alternatives": [ { "layout": {}, "score": {}, "warnings": [] } ]
}
```

`reason` is `no_shape` (the measurements say it fits nowhere) or `no_space`.
The result is deterministic for the same input and `seed`.

## Violations

Hard (the layout is unusable): `unknown_item`, `out_of_bounds`, `overlap`, `fixed_moved`, `does_not_fit`, `cannot_fit_at_span`, `unmeasured`.
Style warnings (`forbidden`, with a `pattern` and the ids involved): `shirimochi` (シリモチ), `heiretsu` (並列), `entotsu` (エントツ), `harakiri` (ハラキリ). Each can be switched off in `options.rules`.

## Options

`beam_width` (16), `branching` (8), `alternatives` (3), `slack`, `max_aspect` (6), `refine_iterations` (400; 0 is the plain beam search), `seed`, `weights`, `rules`.
