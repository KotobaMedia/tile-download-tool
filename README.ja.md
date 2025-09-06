# tile-download-tool

[English](./README.md) | 日本語

[![Crates.io Version](https://img.shields.io/crates/v/tile-download-tool)](https://crates.io/crates/tile-download-tool)

XYZタイルをPMTilesアーカイブに保存します

## 使い方

```
$ tile-download-tool https://example.com/tileset/{z}/{x}/{y}.png example_tileset.pmtiles
```

## インストール

コンパイル済みバイナリは[Releasesページ](https://github.com/KotobaMedia/tile-download-tool/releases)で配布しています。ご利用のアーキテクチャに合ったバイナリをダウンロードし、ターミナルから実行してください。

Rustの開発環境がある場合は、`cargo install tile-download-tool` でもインストールできます。

## オプション

* `--minimum-zoom, -Z`, `--maximum-zoom, -z`: ダウンロード対象のズームレベルを制限します
* `--tile-list [file]`: ダウンロードするタイルのリストファイルを指定します
* `--tile-list-format [format]`: タイルリストファイルの書式を指定します
    * それぞれズーム、x、yの値として `z` `x` `y` が使用されます。
    * `--tile-list-format "z x y"`: 各行が `z x y`（例: `0 0 0`）
    * `--tile-list-format "z/x/y"`: 各行が `z/x/y`
    * `--tile-list-format "z/y/x"`
    * `--tile-list-format "z,x,y"`
    * など。指定したフォーマットから正規表現を生成してマッチングに用います。
* `--bbox, -b`: ダウンロード対象を絞り込む境界ボックス（`min_x,min_y,max_x,max_y` 形式）
* `--concurrency`: 同時ダウンロード数の上限（デフォルト: 10）

全オプションは `--help` で確認できます。
