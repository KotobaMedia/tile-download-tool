# tile-download-tool

Download XYZ tiles in to a PMTiles archive

## Usage

```
$ tile-download-tool https://example.com/tileset/{z}/{x}/{y}.png example_tileset.pmtiles
```

## Options

* `--minimum-zoom` `-Z`, `--maximum-zoom` `-z` - limit the zoom levels to download
* `--tile-list [file]` - a list of tiles to download
* `--tile-list-format [format]` - the format the tile-list file is in
    * `z` `x` `y` will be used as the zoom, x, and y values respectively.
    * `--tile-list-format "z x y"` - lines of `z x y` (for example, `0 0 0`)
    * `--tile-list-format "z/x/y"` - lines of `z/x/y`
    * `--tile-list-format "z/y/x"`
    * `--tile-list-format "z,x,y"`
    * ,etc. A regex will be compiled based on the format and used for matching.
* `--concurrency` - limit the download concurrency (defaults to 10)
