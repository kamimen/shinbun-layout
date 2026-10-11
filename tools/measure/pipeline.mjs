// Maker JSON -> problem -> browser measurements -> shinbun-layout -> placements written back -> render and check.
//   node tools/measure/pipeline.mjs examples/maker-vertical.json out
// Needs a release build of the solve example and a local shinbun-cli (SHINBUN_CLI, default ../../../shinbun-cli).
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { launch, measureElements } from "./measure.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const [input = "examples/maker-vertical.json", outDir = "out"] = process.argv.slice(2);
const cli = resolve(here, process.env.SHINBUN_CLI ?? "../../../shinbun-cli");
const solve = resolve(here, "../../target/release/examples/solve");
mkdirSync(resolve(outDir), { recursive: true });
const out = (name) => resolve(outDir, name);

const { parseMakerJson } = await import(pathToFileURL(`${cli}/src/core/maker-json.ts`));
const { renderPaper } = await import(pathToFileURL(`${cli}/src/core/html.ts`));

const maker = JSON.parse(readFileSync(resolve(input), "utf8"));
const doc = parseMakerJson(readFileSync(resolve(input), "utf8"));
const { paper } = doc;

const ROLE = { nameplate: "nameplate", article: "article", photo: "photo", box: "box", yoko: "yoko", ad: "ad" };
const PRIORITY = { xl: 9, l: 8, m: 6, s: 3 };
const textLength = (html) => html.replace(/<[^>]*>/g, "").length;
const stripped = (it) => (it.kind === "article" ? [...it.body].reduce((n, p) => n + textLength(p), 0) : it.kind === "box" ? it.body.reduce((n, p) => n + textLength(p), 0) : 0);

// Nameplate, horizontal headline and ad keep the places they have in the file; the rest is laid out again.
const KEEP = new Set(["nameplate", "yoko", "ad"]);
const items = doc.items.map((it, i) => {
  const base = { id: `i${i}`, role: ROLE[it.kind], priority: 5, chars: stripped(it) };
  if (KEEP.has(it.kind)) return { ...base, priority: 1, fixed: { x: it.x, w: it.w, y: it.y, span: it.span } };
  if (it.kind === "article") return { ...base, priority: it.isLead ? 10 : PRIORITY[it.headlineSize], headline: it.headlineSize, lead: it.isLead };
  if (it.kind === "photo") return { ...base, priority: 6 };
  return { ...base, priority: 3 };
});
const problem = { paper: { writing: paper.writing, dan: paper.dan, lines: paper.lines, chars: paper.chars }, items };
writeFileSync(out("problem.json"), JSON.stringify(problem, null, 2));

const queries = JSON.parse(execFileSync(solve, ["measurements", out("problem.json")], { encoding: "utf8" }));

/** The item's own markup, placed at (x=1, y=1) with the given size. */
function itemHtml(index, w, span, measureId) {
  const placed = { ...doc.items[index], x: 1, w, y: 1, span };
  const html = renderPaper({ ...doc, items: [placed] });
  const inner = html.match(/<main class="sb-page sb-page--grid">([\s\S]*)<\/main>/)[1];
  return inner.replace(/^<(\w+)/, `<$1 data-id="${measureId}"`);
}

const needsText = (it) => it.kind === "article" || it.kind === "box";
const browser = await launch();
const table = [];
try {
  const jobs = [];
  for (const q of queries) {
    const index = Number(q.id.slice(1));
    if (!needsText(doc.items[index])) {
      table.push({ id: q.id, span: q.span, lines: 1 });
      continue;
    }
    for (let w = 1; w <= paper.lines; w++) jobs.push({ q, index, w });
  }
  const fits = await measureElements(browser, jobs.map((j, n) => itemHtml(j.index, j.w, j.q.span, n)), { ...problem.paper, fs: paper.fs });
  const minimal = new Map();
  jobs.forEach((j, n) => {
    const key = `${j.q.id}:${j.q.span}`;
    if (fits[n] && !minimal.has(key)) minimal.set(key, j.w);
  });
  for (const q of queries.filter((q) => needsText(doc.items[Number(q.id.slice(1))]))) {
    table.push({ id: q.id, span: q.span, lines: minimal.get(`${q.id}:${q.span}`) ?? null });
  }
} finally {
  await browser.close();
}
writeFileSync(out("table.json"), JSON.stringify(table));

const solution = JSON.parse(execFileSync(solve, ["solve", out("problem.json"), out("table.json")], { encoding: "utf8" }));
writeFileSync(out("solution.json"), JSON.stringify(solution, null, 2));

const placed = new Map(solution.layout.placements.map((p) => [p.id, p]));
const laidOut = structuredClone(maker);
laidOut.items.forEach((it, i) => {
  const p = placed.get(`i${i}`);
  if (p) Object.assign(it, { x: p.x, w: p.w, y: p.y, span: p.span });
});
writeFileSync(out("laid-out.json"), JSON.stringify(laidOut, null, 2));

console.log(`unplaced: ${JSON.stringify(solution.unplaced)}`);
console.log(`warnings: ${JSON.stringify(solution.warnings)}`);
console.log(`score: ${solution.score.total.toFixed(2)}  alternatives: ${solution.alternatives.length}`);
console.log(`wrote ${out("laid-out.json")}`);

const css = ["shinbun.min.css", "paper.css"].map((f) => readFileSync(resolve(here, "vendor", f), "utf8")).join("\n");
const page = renderPaper(parseMakerJson(JSON.stringify(laidOut)));
writeFileSync(out("laid-out.html"), `<!doctype html><html lang="ja"><head><meta charset="utf-8"><title>laid-out</title><style>${css}\nbody{margin:0}</style></head><body>${page}</body></html>`);
console.log(`wrote ${out("laid-out.html")}`);
