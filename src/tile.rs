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

    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        let x = self.x() as f32;
        let y = self.y() as f32;
        let n = (1u32 << self.z()) as f32;
        let lon_min = (x / n) * 360.0 - 180.0;
        let lon_max = ((x + 1.0) / n) * 360.0 - 180.0;
        let lat_rad_max = std::f32::consts::PI * (1.0 - 2.0 * y / n);
        let lat_max = lat_rad_max.sinh().atan() * 180.0 / std::f32::consts::PI;
        let lat_rad_min = std::f32::consts::PI * (1.0 - 2.0 * (y + 1.0) / n);
        let lat_min = lat_rad_min.sinh().atan() * 180.0 / std::f32::consts::PI;
        (lon_min, lat_min, lon_max, lat_max)
    }
}

impl std::ops::Deref for Tile {
    type Target = pmtiles::TileCoord;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
