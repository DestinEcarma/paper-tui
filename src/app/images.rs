use std::path::{Path, PathBuf};

use ratatui_image::protocol::StatefulProtocol;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageStatus {
    Unloaded,
    Queued,
    Ready,
    Failed,
}

pub struct ImageItem {
    pub original: PathBuf,
    pub thumbnail: PathBuf,
    pub image: Option<StatefulProtocol>,
    pub status: ImageStatus,
}

pub struct ThumbJob {
    pub index: usize,
    pub src: PathBuf,
    pub thumb: PathBuf,
}

pub struct ThumbResult {
    pub index: usize,
    pub thumb: PathBuf,
    pub ok: bool,
}

pub fn load_images(dir: &Path, cache_dir: &Path) -> std::io::Result<Vec<ImageItem>> {
    std::fs::create_dir_all(cache_dir)?;

    let mut items = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|ext| {
                    matches!(
                        ext.to_ascii_lowercase().as_str(),
                        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif"
                    )
                })
                .unwrap_or(false)
        })
        .map(|original| {
            let stem = original
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("thumb");

            let thumbnail = cache_dir.join(format!("{stem}.png"));

            ImageItem {
                original,
                thumbnail,
                image: None,
                status: ImageStatus::Unloaded,
            }
        })
        .collect::<Vec<_>>();

    items.sort_by(|a, b| a.original.cmp(&b.original));
    Ok(items)
}
