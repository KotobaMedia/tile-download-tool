# tile-download-tool

English | [日本語](./README.ja.md)

[![Crates.io Version](https://img.shields.io/crates/v/tile-download-tool)](https://crates.io/crates/tile-download-tool)

Download XYZ tiles in to a PMTiles archive

## Usage

```
$ tile-download-tool https://example.com/tileset/{z}/{x}/{y}.png example_tileset.pmtiles
```

## Installation

[Compiled binaries are available on the Releases page](https://github.com/KotobaMedia/tile-download-tool/releases). Download the binary for your architecture and run it in a terminal.

If you have a Rust environment installed, you may `cargo install tile-download-tool` as well.

## Options

* `--minimum-zoom, -Z`, `--maximum-zoom, -z` - limit the zoom levels to download
* `--tile-list [file]` - a list of tiles to download
* `--tile-list-format [format]` - the format the tile-list file is in
    * `z` `x` `y` will be used as the zoom, x, and y values respectively.
    * `--tile-list-format "z x y"` - lines of `z x y` (for example, `0 0 0`)
    * `--tile-list-format "z/x/y"` - lines of `z/x/y`
    * `--tile-list-format "z/y/x"`
    * `--tile-list-format "z,x,y"`
    * ,etc. A regex will be compiled based on the format and used for matching.
* `--bbox, -b` - A bounding box in the format "min_x,min_y,max_x,max_y" to filter the downloaded tiles
* `--concurrency` - limit the download concurrency (defaults to 10)

See all options with `--help`
