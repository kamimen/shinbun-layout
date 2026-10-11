# shinbun-layout

日本語の新聞の紙面を自動で組むための、ライブラリです。記事・写真・囲みを渡すと、[shinbun-css](https://github.com/kamimen/shinbun-css) の段組み（グリッド）のどこに置くかを返します。大きさと位置を決めるだけで、文字は、呼び出し側が（ブラウザで）測って、答えを渡します。

<p>
  <img src="docs/images/auto-vertical.png" alt="shinbun-layout で組んだ縦組みの紙面" height="360">
  <img src="docs/images/auto-horizontal.png" alt="shinbun-layout で組んだ横組みの紙面" height="360">
</p>

## しくみ

1. `required_measurements(problem)` が、測る大きさを返します。部品ごと、段数（段）ごとに 1 つの問いです。
2. 呼び出し側が、紙面を描くブラウザで、段数ごとに必要な最小の行数を測って、表を作ります。
3. `solve(problem, table, options)` が、位置を返します。純粋で決定的で、同じ入力と seed なら同じ配置になります。

入力も出力も JSON です（形式は [docs/SPEC.md](docs/SPEC.md)、英語）。問題には文章は入らず、長さと重要度だけです。例は [英語版](README.md) にあります。

`{x, w, y, span}` は、shinbun-css と shinbun-maker で使っている位置の形式です。置けなかった部品は理由つきで `unplaced` に、配置の問題（シリモチ・並列・エントツ・ハラキリ）は警告に入ります。`validate()` は、誰が作った配置でも検査できます。

## 使い方

Rust、WebAssembly（`scripts/build-wasm.sh` で `pkg/web` と `pkg/node` を作ります）、コマンド（`shinbun-layout measurements | solve | validate`）から使えます。コードの例は [英語版](README.md#use-it) にあります。`tools/measure` は、測定用の参考実装（Playwright と Chrome）です。

## ビルド

ソースからビルドします。

```sh
git clone https://github.com/kamimen/shinbun-layout
cd shinbun-layout
cargo build --release          # target/release/shinbun-layout
cargo test
scripts/build-wasm.sh          # wasm32 の環境と wasm-bindgen-cli が必要（版はスクリプトが教えます）
```

## ドキュメント

[kamimen.github.io/shinbun-layout](https://kamimen.github.io/shinbun-layout/)：JSON の形式、配置の探し方、Rust の API（英語）。

[English](README.md)

## ライセンス

MIT
