# shinbun-layout

[shinbun-css](https://github.com/kamimen/shinbun-css) の段組み（グリッド）に、新聞の紙面（Japanese newspaper pages）を自動で組むための Rust クレートです。

記事・写真・囲みを渡すと、shinbun-maker や shinbun-cli と同じ `{x, w, y, span}` の位置を返します。

## しくみ

文字の描画はしません。使い方は 3 手順です。

1. `required_measurements(&problem)` で、測るべき大きさを得る
2. 呼び出し側（ブラウザ。`tools/measure` を参照）が、段数（span）ごとに必要な最小の行数を測る
3. `solve(&problem, &table, &options)` で、位置・置けなかった記事とその理由・評価・警告・別案を得る（純粋で決定的）

`validate()` は、禁止レイアウト（シリモチ・並列・エントツ・ハラキリ）を含め、どんな配置も検査できます。

詳しくは [docs/ALGORITHM.md](docs/ALGORITHM.md)（英語）を参照してください。現在は未公開です。

[English](README.md)

## ライセンス

MIT
