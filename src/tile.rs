#[derive(Clone)]
pub struct Tile(pmtiles::TileCoord);

impl Tile {
    pub fn from_captures(caps: &regex::Captures) -> Self {
        let z = caps["z"].parse().unwrap();
        let x = caps["x"].parse().unwrap();
        let y = caps["y"].parse().unwrap();

        Self::new(z, x, y)
    }

    pub fn new(z: u8, x: u32, y: u32) -> Self {
        Self(pmtiles::TileCoord::new(z, x, y).unwrap())
    }

    pub fn to_id(&self) -> pmtiles::TileId {
        self.0.into()
    }

    pub fn to_string(&self) -> String {
        format!("{}/{}/{}", self.z(), self.x(), self.y())
    }
}

impl std::ops::Deref for Tile {
    type Target = pmtiles::TileCoord;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
