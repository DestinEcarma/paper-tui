use std::{path::Path, sync::mpsc, thread};

use image::ImageReader;

use crate::app::images::{ThumbJob, ThumbResult};

pub fn spawn_thumb_worker(
    rx: mpsc::Receiver<ThumbJob>,
    tx: mpsc::Sender<ThumbResult>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while let Ok(job) = rx.recv() {
            let ok = if job.thumb.exists() {
                true
            } else {
                make_thumbnail(&job.src, &job.thumb).is_ok()
            };

            let _ = tx.send(ThumbResult {
                index: job.index,
                thumb: job.thumb,
                ok,
            });
        }
    })
}

fn make_thumbnail(src: &Path, thumb: &Path) -> anyhow::Result<()> {
    if let Some(parent) = thumb.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let img = ImageReader::open(src)?.decode()?;
    let small = img.thumbnail(320, 180);
    small.save(thumb)?;
    Ok(())
}
