use std::sync::mpsc;

use ratatui_image::picker::Picker;
use ratatui_textarea::TextArea;

use crate::{
    app::images::{ImageItem, ThumbJob, ThumbResult},
    config::Config,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedBlock {
    Search,
    Menu,
    Wallpapers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    PickFolder,
    Refresh,
}

impl MenuAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::PickFolder => "Folder",
            Self::Refresh => "Refresh",
        }
    }
}

pub struct App<'a> {
    pub exit: bool,

    pub config: Config,

    pub focused_block: FocusedBlock,

    pub selected_menu: usize,
    pub selected_image: usize,
    pub scroll_row: usize,

    pub search_input: TextArea<'a>,

    pub picker: Picker,
    pub all_images: Vec<ImageItem>,
    pub filtered: Vec<usize>,

    pub thumb_tx: mpsc::Sender<ThumbJob>,
    pub thumb_rx: mpsc::Receiver<ThumbResult>,
}
