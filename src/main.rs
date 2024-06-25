mod cli;
mod tui;
mod models;

use anyhow::{anyhow, bail, ensure, Result};
use clap::{Parser};
use cli::{RustReplay, SubCommands};
use crossterm::event::{self, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind};
use models::ReplayFile;
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{block::*, *},
};
use serde_json::json;
use std::{
    error::Error,
    fs::{self, Metadata},
    io::{self, Read},
    path::PathBuf,
    time::Duration,
};

#[derive(Debug, Default)]
pub struct App {
    replay_dir: PathBuf,
    replay_files: Vec<ReplayFile>,
    parsing_paths: Vec<PathBuf>,
    parsing_index: usize,
    parsing_progress: f64,
    state: AppState,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum AppState {
    #[default]
    Browsing,
    Parsing,
    Quitting,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut tui::Tui, replay_dir: PathBuf) -> io::Result<()> {
        self.replay_dir = replay_dir;
        self.initialize_parse(); // begin parsing on startup
        
        while self.state != AppState::Quitting {
            match self.state {
                AppState::Browsing => {}, //TODO
                AppState::Parsing => self.step_parse(),
                _ => {}
            }
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render_frame(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.size());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        let timeout = Duration::from_secs_f32(1.0 / 20.0);
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    use KeyCode::*;
                    match key.code {
                        Char(' ') | Enter => self.initialize_parse(),
                        Char('q') | Esc => self.quit(),
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    fn quit(&mut self) {
        self.state = AppState::Quitting;
    }

    fn initialize_parse(&mut self) {
        self.parsing_paths = get_dir_files(&self.replay_dir, vec!["replay"]).unwrap();
        self.parsing_index = 0;
        self.parsing_progress = 0.0;
        self.state = AppState::Parsing;
    }

    fn step_parse(&mut self) {
        if self.parsing_index == ( self.parsing_paths.len() - 1 ) {
            self.finalize_parse();
            return;
        }

        let path = &self.parsing_paths[self.parsing_index];
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap();
        let mut rf = models::ReplayFile::new(name.to_owned(), path.to_path_buf());
        rf.parse_replay();
        self.replay_files.push(rf);

        let total_files = self.parsing_paths.len();
        self.parsing_index += 1;
        self.parsing_progress = self.parsing_index as f64 / total_files as f64;
    }

    fn finalize_parse(&mut self) {
        self.parsing_paths.clear();
        self.parsing_index = 0;
        self.parsing_progress = 0.0;
        self.state = AppState::Browsing;
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Title::from(" RustReplay ".bold());
        
        let instructions = Title::from(Line::from(vec![
            " Parse Replays ".into(),
            "<Space/Enter>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]));
        
        let block = Block::default()
            .title(title.alignment(Alignment::Center))
            .title(
                instructions
                    .alignment(Alignment::Center)
                    .position(Position::Bottom),
            )
            .borders(Borders::ALL)
            .border_set(border::THICK);

        if self.state == AppState::Parsing {
            let total_files = self.parsing_paths.len();
            
            let progress_text = format!(
                "Parsing replays... {}/{}",
                (self.parsing_progress * total_files as f64) as usize,
                total_files
            );

            let gauge = LineGauge::default()
                .block(Block::default().borders(Borders::ALL).title("Progress"))
                .gauge_style(Style::default().fg(Color::Cyan))
                .ratio(self.parsing_progress);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(1)])
                .split(area);

            Paragraph::new(progress_text)
                .alignment(Alignment::Center)
                .render(chunks[0], buf);

            gauge.render(chunks[1], buf);
        } else {
            let items: Vec<ListItem> = self.replay_files.iter()
                .map(|rf| {
                    let mut item = ListItem::new(rf.name.clone());
                    if rf.corrupt {
                        item = item.style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));
                    }
                    item
                })
                .collect();

            let list = List::new(items)
                .block(block)
                .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
                .highlight_symbol(">>")
                .repeat_highlight_symbol(true);
            
            Widget::render(list, area, buf);
        }
    }
}

fn main() -> Result<()> {
    /* Setup CLI */
    let cli = RustReplay::parse();
    let dir = cli.directory.unwrap();

    /* Run function(s) based on the sub(sub)command to be executed */
    match cli.subcommand {
        SubCommands::List { verbose, markdown } => {
            
        }
    };

    /* Setup TUI */
    let mut terminal = tui::init()?;
    let app_result = App::default().run(&mut terminal, dir);
    tui::restore()?;
    
    match app_result {
        Error => Err(anyhow!("")),
        _ => Ok(())
    }
    //     ExitCode::FAILURE
    // } else {
    //     ExitCode::SUCCESS
    // }
}

fn get_dir_files(dir: &PathBuf, exts: Vec<&str>) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    /* Read directory entries */
    let mut paths = fs::read_dir(dir)?
        /* Filter out directory entries which couldn't be read */
        .filter_map(|res| res.ok())
        /* Map the directory entries to paths */
        .map(|dir_entry| dir_entry.path())
        /* Filter out paths based on extensions in `exts`, if `exts` is not empty */
        .filter(|path| {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                exts.is_empty() || exts.contains(&ext)
            } else {
                exts.is_empty() // Include files without extensions if `exts` is empty
            }
        })
        .collect::<Vec<_>>();

    /* Sort paths by modification time (newest to oldest) */
    paths.sort_by(|a, b| {
        let metadata_a = fs::metadata(a).unwrap();
        let metadata_b = fs::metadata(b).unwrap();
        
        /* Compare modification times (descending order) */
        metadata_b.modified().unwrap().cmp(&metadata_a.modified().unwrap())
    });

    Ok(paths)
}
