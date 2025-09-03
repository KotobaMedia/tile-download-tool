use serde::Serialize;

use crate::cli::Cli;

#[derive(Serialize)]
pub struct Metadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub attribution: Option<String>,
}

impl Metadata {
    pub fn new(cli: &Cli) -> Self {
        Self {
            name: cli.name.clone(),
            description: cli.description.clone(),
            attribution: cli.attribution.clone(),
        }
    }
}
