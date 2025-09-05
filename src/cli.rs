use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "tile-download-tool")]
#[command(about = "Download XYZ tiles into a PMTiles archive")]
pub struct Cli {
    /// The URL template for tiles (e.g., https://example.com/tileset/{z}/{x}/{y}.png)
    pub url: String,

    /// Output PMTiles file
    pub output: PathBuf,

    /// Delete the output file if it already exists instead of throwing an error
    #[arg(long, short, default_value_t = false)]
    pub force: bool,

    /// Name of the tileset (for PMTiles metadata)
    #[arg(long, short = 'n')]
    pub name: Option<String>,

    /// Description of the tileset (for PMTiles metadata)
    #[arg(long, short = 'N')]
    pub description: Option<String>,

    /// Attribution information for the tileset (for PMTiles metadata)
    #[arg(long, short = 'A')]
    pub attribution: Option<String>,

    /// Maximum zoom level to download
    #[arg(long, short = 'z', default_value_t = 14)]
    pub maximum_zoom: u8,

    /// Minimum zoom level to download
    #[arg(long, short = 'Z', default_value_t = 0)]
    pub minimum_zoom: u8,

    /// File containing a list of tiles to download
    #[arg(long)]
    pub tile_list: Option<String>,

    /// Format of the tile list file (e.g., "z x y", "z/x/y")
    #[arg(long, default_value = "z/x/y")]
    pub tile_list_format: String,

    /// A bounding box in the format "min_x,min_y,max_x,max_y" to filter the downloaded tiles by
    #[arg(long, short)]
    pub bbox: Option<String>,

    /// Limit the download concurrency
    #[arg(long, default_value_t = 10)]
    pub concurrency: usize,
}
