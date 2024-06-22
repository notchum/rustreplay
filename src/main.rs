mod cli;

use anyhow::{anyhow, bail, ensure, Result};
use boxcars::{ParseError, Replay, HeaderProp};
use clap::{Parser};
use cli::{RustReplay, SubCommands};
use colored::{ColoredString, Colorize};
use indicatif::ProgressBar;
use serde_json::json;
use std::{
    error::Error,
    fs::{self, Metadata},
    io::{self, Read},
    path::PathBuf,
    process::ExitCode,
};

fn main() -> ExitCode {
    let cli = RustReplay::parse();

    if let Err(err) = actual_main(cli) {
        if !err.to_string().is_empty() {
            eprintln!("{}", err.to_string().red().bold());
            if err
                .to_string()
                .to_lowercase()
                .contains("error trying to connect")
                || err
                    .to_string()
                    .to_lowercase()
                    .contains("error sending request")
            {
                eprintln!(
                    "{}",
                    "Verify that you are connnected to the internet"
                        .yellow()
                        .bold()
                );
            }
        }
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn actual_main(mut cli_app: RustReplay) -> Result<()> {
    let mut ret = false;
    
    // Run function(s) based on the sub(sub)command to be executed
    match cli_app.subcommand {
        SubCommands::List { verbose, markdown } => {
            let dir = "/home/chum/.local/share/Steam/steamapps/compatdata/252950/pfx/drive_c/users/steamuser/AppData/Roaming/bakkesmod/bakkesmod/data/replays";

            let paths = get_dir_files(dir, vec!["replay"]).unwrap();

            /* Create a progress bar */
            println!("Parsing replay files...");
            let pb = ProgressBar::new(paths.len().try_into().unwrap());

            for (i, path) in paths.iter().enumerate() {
                /* Extract the name of the file from the path */
                let name = path.file_stem().and_then(|s| s.to_str()).unwrap();
                // println!("{}. {:?}", i, name);
                
                /* Attempt to read the entire file into a buffer */
                let buffer = fs::read(path.as_path())?;
                
                /* Attempt to parse the replay file from the buffer*/
                match parse_rl(&buffer) {
                    Ok(replay) => {
                        let mut record_fps: Option<f32> = None;
                        let mut num_frames: Option<i32> = None;

                        // Iterate through properties to find RecordFPS and NumFrames
                        for (name, value) in &replay.properties {
                            match name.as_str() {
                                "RecordFPS" => {
                                    if let HeaderProp::Float(fps) = value {
                                        record_fps = Some(*fps);
                                    }
                                }
                                "NumFrames" => {
                                    if let HeaderProp::Int(frames) = value {
                                        num_frames = Some(*frames);
                                    }
                                }
                                _ => {}
                            }
                        }

                        // Calculate raw seconds if both RecordFPS and NumFrames are available
                        if let (Some(record_fps), Some(num_frames)) = (record_fps, num_frames) {
                            let raw_seconds = num_frames as f32 / record_fps;
                            // println!("Raw seconds: {}", raw_seconds);
                        } else {
                            // println!("Unable to calculate raw seconds. Missing RecordFPS or NumFrames.");
                        }
                    },
                    Err(err) => {
                        // Handle the parsing error
                        // eprintln!("Error parsing replay: {}", err);
                    }
                }    

                // let obj = json!(&replay);
                // println!("{}", serde_json::to_string_pretty(&obj).unwrap());
                // break;
                pb.inc(1);
            }

            pb.finish_with_message("Finished parsing replays!");

            // let profile = get_active_profile(&mut config)?;
            // check_empty_profile(profile)?;
            // if verbose {
            //     subcommands::list::verbose(modrinth, curseforge, profile, markdown).await?;
            // } else {
            //     println!(
            //         "{} {} on {} {}\n",
            //         profile.name.bold(),
            //         format!("({} mods)", profile.mods.len()).yellow(),
            //         format!("{:?}", profile.mod_loader).purple(),
            //         profile.game_version.green(),
            //     );
            //     for mod_ in &profile.mods {
            //         println!(
            //             "{:20}  {}",
            //             match &mod_.identifier {
            //                 ModIdentifier::CurseForgeProject(id) =>
            //                     format!("{} {:8}", "CF".red(), id.to_string().dimmed()),
            //                 ModIdentifier::ModrinthProject(id) =>
            //                     format!("{} {:8}", "MR".green(), id.dimmed()),
            //                 ModIdentifier::GitHubRepository(_) => "GH".purple().to_string(),
            //             },
            //             match &mod_.identifier {
            //                 ModIdentifier::ModrinthProject(_)
            //                 | ModIdentifier::CurseForgeProject(_) => mod_.name.bold().to_string(),
            //                 ModIdentifier::GitHubRepository(id) =>
            //                     format!("{}/{}", id.0.dimmed(), id.1.bold()),
            //             },
            //         );
            //     }
            // }
        }
    };

    if ret {
        Err(anyhow!(""))
    } else {
        Ok(())
    }
}

fn parse_rl(data: &[u8]) -> Result<Replay, ParseError> {
    boxcars::ParserBuilder::new(data)
        .must_parse_network_data()
        .parse()
}

fn get_dir_files(dir: &str, exts: Vec<&str>) -> Result<Vec<PathBuf>, Box<dyn Error>> {
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
