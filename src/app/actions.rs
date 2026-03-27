use std::{borrow::Cow, ops::Add, path::Path, sync::mpsc, time::Duration};

use image::ImageReader;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode, KeyEvent},
};
use ratatui_image::picker::Picker;
use ratatui_textarea::TextArea;

use crate::{
    app::{
        App,
        images::{ImageStatus, ThumbJob, ThumbResult, load_images},
        render::RenderExt,
        state::{FocusedBlock, MenuAction},
        worker::spawn_thumb_worker,
    },
    config::Config,
    util,
};

impl Default for App<'_> {
    fn default() -> Self {
        let (thumb_tx, worker_rx) = mpsc::channel::<ThumbJob>();
        let (worker_tx, thumb_rx) = mpsc::channel::<ThumbResult>();

        spawn_thumb_worker(worker_rx, worker_tx);

        let config = Config::load().unwrap_or_default();

        Self {
            exit: false,
            config,
            focused_block: FocusedBlock::Wallpapers,
            selected_menu: 0,
            selected_image: 0,
            scroll_row: 0,
            search_input: TextArea::default(),
            picker: Picker::from_query_stdio().expect("failed to create picker"),
            all_images: Vec::new(),
            filtered: Vec::new(),
            thumb_tx,
            thumb_rx,
        }
    }
}

impl App<'_> {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        self.refresh_images()?;

        while !self.exit {
            self.poll_thumb_results();
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    pub fn handle_events(&mut self) -> std::io::Result<()> {
        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key_event) = event::read()?
            && key_event.is_press()
        {
            self.handle_key_event(key_event)?;
        }

        Ok(())
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> std::io::Result<()> {
        match self.focused_block {
            FocusedBlock::Search => self.handle_search_keys(key_event),
            FocusedBlock::Menu => self.handle_menu_keys(key_event),
            FocusedBlock::Wallpapers => self.handle_wallpaper_keys(key_event),
        }
    }

    fn handle_search_keys(&mut self, key_event: KeyEvent) -> std::io::Result<()> {
        match key_event.code {
            KeyCode::Tab => self.focus_next(),
            KeyCode::Esc => self.focused_block = FocusedBlock::Wallpapers,
            _ => {
                self.search_input.input(key_event);
                self.refilter();
            }
        }
        Ok(())
    }

    fn handle_menu_keys(&mut self, key_event: KeyEvent) -> std::io::Result<()> {
        match key_event.code {
            KeyCode::Char('q') => self.exit = true,
            KeyCode::Tab => self.focus_next(),
            KeyCode::Char('h') | KeyCode::Left => {
                self.selected_menu = self.selected_menu.saturating_sub(1);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.selected_menu = (self.selected_menu + 1).min(self.menu_actions().len() - 1);
            }
            KeyCode::Char(' ') | KeyCode::Enter => self.activate_menu_action()?,
            _ => {}
        }
        Ok(())
    }

    fn handle_wallpaper_keys(&mut self, key_event: KeyEvent) -> std::io::Result<()> {
        if self.filtered.is_empty() {
            return match key_event.code {
                KeyCode::Char('q') => {
                    self.exit = true;
                    Ok(())
                }
                KeyCode::Tab => {
                    self.focus_next();
                    Ok(())
                }
                KeyCode::Char('r') => self.refresh_images(),
                _ => Ok(()),
            };
        }

        match key_event.code {
            KeyCode::Char('q') => self.exit = true,
            KeyCode::Tab => self.focus_next(),
            KeyCode::Char('h') | KeyCode::Left => {
                self.selected_image = self.selected_image.saturating_sub(1);
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.selected_image = self.selected_image.add(3).min(self.filtered.len() - 1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_image = self.selected_image.saturating_sub(3);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.selected_image = self.selected_image.add(1).min(self.filtered.len() - 1);
            }
            KeyCode::Char(' ') | KeyCode::Enter => self.set_wallpaper()?,
            KeyCode::Char('r') => self.refresh_images()?,
            _ => {}
        }

        Ok(())
    }

    fn focus_next(&mut self) {
        self.focused_block = match self.focused_block {
            FocusedBlock::Search => FocusedBlock::Menu,
            FocusedBlock::Menu => FocusedBlock::Wallpapers,
            FocusedBlock::Wallpapers => FocusedBlock::Search,
        };
    }

    pub fn search_query(&self) -> String {
        self.search_input.lines().join("").trim().to_string()
    }

    pub fn refilter(&mut self) {
        let query = self.search_query().to_lowercase();

        self.filtered = self
            .all_images
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                if query.is_empty() {
                    return true;
                }

                item.original
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|name| name.to_lowercase().contains(&query))
                    .unwrap_or(false)
            })
            .map(|(idx, _)| idx)
            .collect();

        self.selected_image = if self.filtered.is_empty() {
            0
        } else {
            self.selected_image.min(self.filtered.len() - 1)
        };
    }

    pub fn menu_actions(&self) -> [MenuAction; 2] {
        [MenuAction::PickFolder, MenuAction::Refresh]
    }

    pub fn activate_menu_action(&mut self) -> std::io::Result<()> {
        match self.menu_actions()[self.selected_menu] {
            MenuAction::PickFolder => self.pick_folder()?,
            MenuAction::Refresh => self.refresh_images()?,
        }

        Ok(())
    }

    pub fn pick_folder(&mut self) -> std::io::Result<()> {
        let Some(dir) = rfd::FileDialog::new()
            .set_directory(&self.config.wallpapers_dir)
            .pick_folder()
        else {
            return Ok(());
        };

        self.config.wallpapers_dir = dir;
        self.config.save()?;
        self.refresh_images()
    }

    pub fn refresh_images(&mut self) -> std::io::Result<()> {
        let cache_dir = util::cache_dir();

        self.all_images = load_images(&self.config.wallpapers_dir, &cache_dir)?;
        self.filtered = (0..self.all_images.len()).collect();
        self.selected_image = 0;
        self.scroll_row = 0;

        Ok(())
    }

    pub fn set_wallpaper(&self) -> std::io::Result<()> {
        if self.filtered.is_empty() {
            return Ok(());
        }

        let real_idx = self.filtered[self.selected_image];
        self.run_post_command(&self.all_images[real_idx].original)
    }

    pub fn run_post_command(&self, wallpaper: &Path) -> std::io::Result<()> {
        let Some(cmd) = &self.config.post_command else {
            return Ok(());
        };

        let quoted =
            shell_escape::unix::escape(Cow::from(wallpaper.to_string_lossy().into_owned()));
        let cmd = cmd.replace("{wallpaper}", &quoted);

        std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        Ok(())
    }

    pub fn queue_visible_thumbs(&mut self, start: usize, end: usize) {
        for filtered_idx in start..end {
            let real_idx = self.filtered[filtered_idx];
            let item = &mut self.all_images[real_idx];

            if item.status != ImageStatus::Unloaded {
                continue;
            }

            item.status = ImageStatus::Queued;

            if self
                .thumb_tx
                .send(ThumbJob {
                    index: real_idx,
                    src: item.original.clone(),
                    thumb: item.thumbnail.clone(),
                })
                .is_err()
            {
                item.status = ImageStatus::Failed;
            }
        }
    }

    pub fn poll_thumb_results(&mut self) {
        while let Ok(msg) = self.thumb_rx.try_recv() {
            if !msg.ok {
                self.all_images[msg.index].status = ImageStatus::Failed;
                continue;
            }

            let Ok(reader) = ImageReader::open(&msg.thumb) else {
                self.all_images[msg.index].status = ImageStatus::Failed;
                continue;
            };

            let Ok(img) = reader.decode() else {
                self.all_images[msg.index].status = ImageStatus::Failed;
                continue;
            };

            let protocol = self.picker.new_resize_protocol(img);
            self.all_images[msg.index].image = Some(protocol);
            self.all_images[msg.index].status = ImageStatus::Ready;
        }
    }
}
