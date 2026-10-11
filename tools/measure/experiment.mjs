// Checks the premise of shinbun-layout: whether an article fits depends only on its shape (span, w), not on where it is placed.
import { writeFileSync } from "node:fs";
import { PAPER, launch, measureFits } from "./measure.mjs";

const para = (s, n) => s.repeat(n);
const ARTICLES = {
  plain: {
    headline: "夜9時まで開館",
    headlineSize: "l",
    body: [
      para("架空の町の図書館が、あすから平日の夜九時まで開館する。仕事帰りの住民から要望が多かった。", 3),
      para("初日の午後六時からは、地元の小学生による朗読会が開かれる。参加を申し込んだのは約40人で、会場は閲覧室だ。", 3),
      para("館長は「夜の図書館は、昼とは違う静けさがある。ゆっくり本と向き合ってほしい」と話している。", 2),
    ],
  },
  bigHeadline: {
    headline: "町の図書館、夜も開館へ",
    headlineSize: "xl",
    body: [para("架空の町の図書館が、あすから平日の夜九時まで開館する。仕事帰りの住民から要望が多かった。", 6)],
  },
};

const SPANS = [1, 2, 3, 4, 5, 6, 8, 10];
const XS = [1, 20];
const ys = (span) => [...new Set([1, PAPER.dan - span + 1])];

const browser = await launch();
const report = [];
let disagreements = 0;
let nonMonotone = 0;
try {
  for (const [name, article] of Object.entries(ARTICLES)) {
    for (const span of SPANS) {
      const configs = [];
      for (const x of XS) for (const y of ys(span)) for (let w = 1; x + w - 1 <= PAPER.lines; w++) configs.push({ article, at: { x, w, y, span }, key: `${x},${y}` });
      const fits = await measureFits(browser, configs);
      const byPos = new Map();
      configs.forEach((c, i) => {
        if (!byPos.has(c.key)) byPos.set(c.key, new Map());
        byPos.get(c.key).set(c.at.w, fits[i]);
      });
      const minW = {};
      for (const [pos, m] of byPos) {
        const ws = [...m.keys()].sort((a, b) => a - b);
        const first = ws.find((w) => m.get(w));
        minW[pos] = first ?? null;
        const afterFirst = first === undefined ? [] : ws.filter((w) => w > first);
        if (afterFirst.some((w) => !m.get(w))) nonMonotone++;
      }
      const common = [...byPos.values()].reduce((acc, m) => (acc === null ? new Set(m.keys()) : new Set([...acc].filter((w) => m.has(w)))), null);
      let differing = 0;
      for (const w of common) {
        const values = new Set([...byPos.values()].map((m) => m.get(w)));
        if (values.size > 1) differing++;
      }
      disagreements += differing;
      report.push({ article: name, span, minW, comparedWidths: common.size, positionsDisagree: differing });
    }
  }
} finally {
  await browser.close();
}

writeFileSync(new URL(`./experiment-${process.env.BROWSER ?? "chromium"}-${PAPER.writing}.json`, import.meta.url), JSON.stringify({ paper: PAPER, report, disagreements, nonMonotone }, null, 2));
for (const r of report) console.log(`${r.article.padEnd(12)} span=${String(r.span).padStart(2)}  minW=${JSON.stringify(r.minW)}  compared=${r.comparedWidths}  disagree=${r.positionsDisagree}`);
console.log(`\ntotal disagreements across positions: ${disagreements}`);
console.log(`non-monotone cases (fits at w, but not at a larger w): ${nonMonotone}`);
