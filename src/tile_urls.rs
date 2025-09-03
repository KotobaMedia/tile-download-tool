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
