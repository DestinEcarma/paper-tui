mod app;
mod config;
mod util;

fn main() -> std::io::Result<()> {
    ratatui::run(|terminal| app::App::default().run(terminal))?;
    Ok(())
}
