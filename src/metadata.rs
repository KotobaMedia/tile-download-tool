use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::cli::Cli;

#[derive(Serialize)]
pub struct Metadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub attribution: Option<String>,
    pub version: Option<String>,
}

impl Metadata {
    pub fn new(cli: &Cli) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("It can't be before 1970...")
            .as_secs();
        let version = format!("1.0.0.{}", now);

        Self {
            name: cli.name.clone(),
            description: cli.description.clone(),
            attribution: cli.attribution.clone(),
            version: Some(version),
        }
    }
}
