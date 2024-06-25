mod cli;
mod tui;
mod models;

use anyhow::{anyhow, bail, ensure, Result};
use boxcars::{ParseError, Replay, HeaderProp};
use clap::{Parser};
use cli::{RustReplay, SubCommands};
use crossterm::event::{self, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind};
use indicatif::ProgressBar;
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
    process::ExitCode,
};

#[derive(Debug, Default)]
pub struct App {
    replay_dir: PathBuf,
    replay_files: Vec<ReplayFile>,
    exit: bool,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut tui::Tui, replay_dir: PathBuf) -> io::Result<()> {
        self.replay_dir = replay_dir;
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render_frame(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.size());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Up => self.list(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn list(&mut self) {
        /* Grab the paths of each replay file */
        let paths = get_dir_files(&self.replay_dir, vec!["replay"]).unwrap();

        /* Create a progress bar */
        // println!("Parsing replay files...");
        // let pb = ProgressBar::new(paths.len().try_into().unwrap());

        for (i, path) in paths.iter().enumerate() {
            /* Extract the name of the file from the path */
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap();
            // println!("{}. {:?}", i, name);

            /* Create the replay file */
            let mut rf = models::ReplayFile::new(name.to_owned(), path.to_path_buf());
            
            /* Parse the replay */
            rf.parse_replay();

            self.replay_files.push(rf);

            // let obj = json!(&replay);
            // println!("{}", serde_json::to_string_pretty(&obj).unwrap());
            // break;
            // pb.inc(1);
        }

        // pb.finish_with_message("Finished parsing replays!");
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Title::from(" RustReplay ".bold());
        
        let instructions = Title::from(Line::from(vec![
            " List ".into(),
            "<Up>".blue().bold(),
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
