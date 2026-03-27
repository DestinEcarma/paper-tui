use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Paragraph, Tabs},
};
use ratatui_image::{Resize, StatefulImage};

use crate::app::{App, images::ImageStatus, state::FocusedBlock};

pub trait RenderExt {
    fn draw(&mut self, frame: &mut Frame);
}

impl RenderExt for App<'_> {
    fn draw(&mut self, frame: &mut Frame) {
        let [top_area, wallpapers_area, help_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .margin(1)
        .areas(frame.area());

        let [search_area, menu_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(20)]).areas(top_area);

        self.render_search(frame, search_area);
        self.render_menu(frame, menu_area);
        self.render_wallpapers(frame, wallpapers_area);
        self.render_help(frame, help_area);
    }
}

impl App<'_> {
    pub fn pane_block<'a>(&self, title: &'a str, focused: bool) -> Block<'a> {
        Block::default()
            .title(format!(" {title} "))
            .borders(Borders::ALL)
            .border_style(if focused {
                Style::default().fg(Color::Green).bold()
            } else {
                Style::default()
            })
            .border_type(if focused {
                BorderType::Thick
            } else {
                BorderType::default()
            })
    }

    pub fn render_search(&mut self, frame: &mut Frame, area: Rect) {
        self.search_input
            .set_block(self.pane_block("Search", self.focused_block == FocusedBlock::Search));
        frame.render_widget(&self.search_input, area);
    }

    pub fn render_menu(&self, frame: &mut Frame, area: Rect) {
        let actions = self.menu_actions().map(|item| item.label());
        let focused = self.focused_block == FocusedBlock::Menu;

        let tabs = Tabs::new(actions)
            .block(self.pane_block("Menu", focused))
            .select(self.selected_menu)
            .highlight_style(if focused {
                Style::default().fg(Color::Black).bg(Color::Green).bold()
            } else {
                Style::default()
            });

        frame.render_widget(tabs, area);
    }

    pub fn render_wallpapers(&mut self, frame: &mut Frame, area: Rect) {
        let block = self.pane_block("Wallpapers", self.focused_block == FocusedBlock::Wallpapers);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let columns = 3;
        let cell_height = 12u16;
        let visible_rows = usize::max(1, inner.height as usize / cell_height as usize);

        if self.filtered.is_empty() {
            frame.render_widget(Paragraph::new("No wallpapers found").centered(), inner);
            return;
        }

        let selected_row = self.selected_image / columns;
        let total_rows = self.filtered.len().div_ceil(columns);

        let scroll_row = if selected_row < visible_rows {
            0
        } else {
            selected_row + 1 - visible_rows
        }
        .min(total_rows.saturating_sub(visible_rows));

        let row_areas =
            Layout::vertical(vec![Constraint::Length(cell_height); visible_rows]).split(inner);

        for (visible_row_idx, row_area) in row_areas.iter().enumerate() {
            let actual_row = scroll_row + visible_row_idx;
            let start_idx = actual_row * columns;

            if start_idx >= self.filtered.len() {
                break;
            }

            let cols = Layout::horizontal([
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Fill(1),
            ])
            .split(*row_area);

            for col in 0..columns {
                let filtered_idx = start_idx + col;
                if filtered_idx >= self.filtered.len() {
                    break;
                }

                let real_idx = self.filtered[filtered_idx];
                let item = &mut self.all_images[real_idx];

                let focused = filtered_idx == self.selected_image;
                let cell = Block::default()
                    .borders(Borders::ALL)
                    .border_style(if focused {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    });

                let inner_cell = cell.inner(cols[col]);
                frame.render_widget(cell, cols[col]);

                match item.status {
                    ImageStatus::Ready => {
                        if let Some(image) = item.image.as_mut() {
                            frame.render_stateful_widget(
                                StatefulImage::new().resize(Resize::Fit(None)),
                                inner_cell,
                                image,
                            );
                        }
                    }
                    ImageStatus::Queued | ImageStatus::Unloaded => {
                        frame.render_widget(
                            Paragraph::new(vec![
                                Line::from("Loading").bold(),
                                Line::from(
                                    item.original
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy(),
                                ),
                            ])
                            .centered(),
                            inner_cell,
                        );
                    }
                    ImageStatus::Failed => {
                        frame.render_widget(Paragraph::new("Failed").centered().red(), inner_cell);
                    }
                }
            }
        }

        let start = scroll_row * columns;
        let end = ((scroll_row + visible_rows) * columns).min(self.filtered.len());
        self.queue_visible_thumbs(start, end);
    }

    pub fn render_help(&self, frame: &mut Frame, area: Rect) {
        let lines = match self.focused_block {
            FocusedBlock::Menu => vec![Line::from(vec![
                Span::from("h,←").bold(),
                Span::from(" Left"),
                Span::from(" | "),
                Span::from("l,→").bold(),
                Span::from(" Right"),
                Span::from(" | "),
                Span::from("󱁐,󰌑").bold(),
                Span::from(" Activate"),
                Span::from(" | "),
                Span::from("Tab").bold(),
                Span::from(" Next pane"),
            ])],
            FocusedBlock::Wallpapers => vec![Line::from(vec![
                Span::from("h,←").bold(),
                Span::from(" Left"),
                Span::from(" | "),
                Span::from("k,↑").bold(),
                Span::from(" Up"),
                Span::from(" | "),
                Span::from("j,↓").bold(),
                Span::from(" Down"),
                Span::from(" | "),
                Span::from("l,→").bold(),
                Span::from(" Right"),
                Span::from(" | "),
                Span::from("󱁐,󰌑").bold(),
                Span::from(" Set wallpaper"),
                Span::from(" | "),
                Span::from("r").bold(),
                Span::from(" Refresh"),
                Span::from(" | "),
                Span::from("Tab").bold(),
                Span::from(" Next pane"),
            ])],
            FocusedBlock::Search => vec![Line::from(vec![
                Span::from("Esc").bold(),
                Span::from(" Exit search"),
                Span::from(" | "),
                Span::from("Tab").bold(),
                Span::from(" Next pane"),
            ])],
        };

        frame.render_widget(Paragraph::new(lines).centered().blue(), area);
    }
}
