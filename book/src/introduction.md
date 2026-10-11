# shinbun-layout

Automatic page layout for Japanese newspapers (新聞の紙面) on the column grid of [shinbun-css](https://github.com/kamimen/shinbun-css). Give it articles, photos and boxes; it returns where each one goes. It decides sizes and positions only; the text is measured by your host (a browser) and the answers are passed in.

![A vertical page laid out by shinbun-layout](images/auto-vertical.png)
![A horizontal page laid out by shinbun-layout](images/auto-horizontal.png)

1. `required_measurements(problem)` says which sizes to measure: one question per item and column count (段).
2. Your host measures, in the browser that will draw the page, the least number of lines each item needs at each column count, and builds a table.
3. `solve(problem, table, options)` returns the placements. It is pure and deterministic: the same input and seed give the same layout.
