import { readFileSync } from "node:fs";
import { chromium } from "playwright-core";

const vendor = (name) => readFileSync(new URL(`./vendor/${name}`, import.meta.url), "utf8");
const CSS = `${vendor("shinbun.min.css")}\n${vendor("paper.css")}\nbody{margin:0}`;
const FIND_OVERFLOW = vendor("overflow.js").replace("export function", "function");

export const PAPER = { dan: 10, chars: 10, lines: 60, fs: 0.72 };

const esc = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

export function articleHtml(article, at, id) {
  const attrs = ["x", "w", "y", "span"].map((k) => `data-sb-${k}="${at[k]}"`).join(" ");
  const body = article.body.map((p) => `<p>${esc(p)}</p>`).join("");
  return `<article class="sb-article" data-id="${id}" ${attrs}><h2 class="sb-headline sb-headline--${article.headlineSize}">${esc(article.headline)}</h2>${body}</article>`;
}

export function pageHtml(articles, paper = PAPER) {
  const style = `--sb-chars:${paper.chars};--sb-lines:${paper.lines};--sb-fs:${paper.fs}rem;--sb-dan:${paper.dan}`;
  return `<!doctype html><html lang="ja"><head><meta charset="utf-8"><style>${CSS}</style></head><body><div class="sb-paper" data-sb-writing="vertical" style="${style}"><main class="sb-page sb-page--grid">${articles.join("")}</main></div></body></html>`;
}

/** Opens a browser. Uses the installed Chrome unless CHROME_PATH is set. */
export async function launch() {
  const executablePath = process.env.CHROME_PATH;
  return chromium.launch(executablePath ? { executablePath } : { channel: "chrome" });
}

/** Renders every configuration on one page and returns which ones overflow. */
export async function measureFits(browser, configs, paper = PAPER) {
  const page = await browser.newPage({ viewport: { width: 1600, height: 1200 } });
  try {
    await page.setContent(pageHtml(configs.map((c, i) => articleHtml(c.article, c.at, i)), paper), { waitUntil: "load" });
    await page.evaluate(() => document.fonts.ready.then(() => undefined));
    const overflowing = await page.evaluate(`(() => { ${FIND_OVERFLOW}; return findOverflow(document).map((el) => Number(el.dataset.id)); })()`);
    const bad = new Set(overflowing);
    return configs.map((_, i) => !bad.has(i));
  } finally {
    await page.close();
  }
}

const OVERFLOW_ANY = `(() => {
  const bad = [];
  for (const el of document.querySelectorAll(".sb-page--grid > [data-id]")) {
    const box = el.getBoundingClientRect();
    const vertical = getComputedStyle(el).writingMode.startsWith("vertical");
    const isBox = el.classList.contains("sb-box");
    const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
    const range = document.createRange();
    let over = false;
    while (!over && walker.nextNode()) {
      range.selectNodeContents(walker.currentNode);
      for (const r of range.getClientRects()) {
        if (r.width > 0 && r.height > 0 && (isBox ? (vertical ? r.left < box.left - 1 : r.bottom > box.bottom + 1) : vertical ? r.bottom > box.bottom + 1 : r.right > box.right + 1)) { over = true; break; }
      }
    }
    if (over) bad.push(Number(el.dataset.id));
  }
  return bad;
})()`;

/**
 * Renders many item elements on one page and says which of them overflow.
 * Each entry is the item's own markup, already carrying its placement and a numeric `data-id`.
 */
export async function measureElements(browser, elements, paper = PAPER) {
  const page = await browser.newPage({ viewport: { width: 1600, height: 1200 } });
  try {
    await page.setContent(pageHtml(elements, paper), { waitUntil: "load" });
    await page.evaluate(() => document.fonts.ready.then(() => undefined));
    const bad = new Set(await page.evaluate(OVERFLOW_ANY));
    return elements.map((_, i) => !bad.has(i));
  } finally {
    await page.close();
  }
}
