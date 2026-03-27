use std::path::PathBuf;

pub fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .expect("could not determine cache dir")
        .join("paper-tui")
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .expect("could not determine config dir")
        .join("paper-tui")
}

pub fn config_file() -> PathBuf {
    config_dir().join("config.toml")
}
