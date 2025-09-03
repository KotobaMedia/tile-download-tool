use anyhow::Result;
use regex::Regex;

pub fn compile_tile_format(format: &str) -> Result<Regex> {
    let regex_str = format
        .replace("z", r"(?<z>\d+)")
        .replace("x", r"(?<x>\d+)")
        .replace("y", r"(?<y>\d+)");
    let regex = Regex::new(&format!("^{}$", regex_str))?;
    Ok(regex)
}
