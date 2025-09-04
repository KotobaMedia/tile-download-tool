use crate::tile::Tile;

pub struct TileUrl {
    url: String,
    // tile: Tile,
}

impl TileUrl {
    pub fn from_template(url_template: &str, tile: Tile) -> Self {
        let url = url_template
            .replace("{z}", &tile.z().to_string())
            .replace("{x}", &tile.x().to_string())
            .replace("{y}", &tile.y().to_string());
        TileUrl {
            url,
            // tile
        }
    }

    pub fn url(self) -> String {
        self.url
    }
}

/// Infer the tile format/extension from the end of the URL path using the `url` crate.
/// Query strings are ignored. If no extension is found, defaults to `png`.
pub fn infer_tile_format(url_template: &str) -> String {
    // Replace common placeholders with dummy values so the URL parses
    let dummy = url_template
        .replace("{z}", "0")
        .replace("{x}", "0")
        .replace("{y}", "0");

    if let Ok(parsed) = url::Url::parse(&dummy)
        && let Some(seg) = parsed.path_segments().and_then(|mut s| s.next_back())
        && let Some(dot) = seg.rfind('.')
    {
        let ext = &seg[dot + 1..];
        // Map aliases like pbf -> mvt
        return match ext.to_ascii_lowercase().as_str() {
            "pbf" => "mvt".to_string(),
            other => other.to_string(),
        };
    }

    "png".to_string()
}

#[cfg(test)]
mod tests {
    use super::infer_tile_format;

    #[test]
    fn infers_png_simple() {
        let ext = infer_tile_format("https://example.com/tiles/1/2/3.png");
        assert_eq!(ext, "png");
    }

    #[test]
    fn infers_jpeg_from_template() {
        let ext = infer_tile_format("https://example.com/tiles/{z}/{x}/{y}.jpg");
        assert_eq!(ext, "jpg");
    }

    #[test]
    fn ignores_query_string() {
        let ext = infer_tile_format("https://example.com/tiles/1/2/3.webp?token=abc&cache=bust");
        assert_eq!(ext, "webp");
    }

    #[test]
    fn maps_pbf_to_mvt() {
        let ext = infer_tile_format("https://example.com/tiles/1/2/3.pbf");
        assert_eq!(ext, "mvt");
    }

    #[test]
    fn defaults_to_png_when_missing() {
        // No extension – should default to png
        let ext = infer_tile_format("https://example.com/tiles/{z}/{x}/{y}");
        assert_eq!(ext, "png");
    }

    #[test]
    fn normalizes_uppercase_extension() {
        let ext = infer_tile_format("https://example.com/tiles/1/2/3.PNG");
        assert_eq!(ext, "png");
    }
}
