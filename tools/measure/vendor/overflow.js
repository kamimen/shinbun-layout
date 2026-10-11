// Copied from shinbun-components (src/internal/overflow.ts, compiled). See VERSION.
// Runs inside the page, so it must stay a single self-contained function.
export function findOverflow(root) {
  return [...root.querySelectorAll(".sb-page--grid > .sb-article")].filter((el) => {
    const doc = el.ownerDocument;
    const box = el.getBoundingClientRect();
    const vertical = getComputedStyle(el).writingMode.startsWith("vertical");
    const walker = doc.createTreeWalker(el, NodeFilter.SHOW_TEXT);
    const range = doc.createRange();
    while (walker.nextNode()) {
      range.selectNodeContents(walker.currentNode);
      for (const r of range.getClientRects()) {
        if (r.width > 0 && r.height > 0 && (vertical ? r.bottom > box.bottom + 1 : r.right > box.right + 1)) return true;
      }
    }
    return false;
  });
}
