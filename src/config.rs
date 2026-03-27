use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::util;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub wallpapers_dir: PathBuf,
    pub post_command: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            wallpapers_dir: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/"))
                .join("Pictures"),
            post_command: None,
        }
    }
}

impl Config {
    pub fn save(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(util::config_dir())?;

        let toml = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(util::config_file(), toml)?;

        Ok(())
    }

    pub fn load() -> std::io::Result<Self> {
        let path = util::config_file();

        if !path.exists() {
            return Ok(Self::default());
        }

        let text = std::fs::read_to_string(path)?;
        let cfg = toml::from_str(&text)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Ok(cfg)
    }
}
