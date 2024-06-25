use boxcars::{HeaderProp, ParseError, Replay};
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Debug)]
pub struct ReplayFile {
    pub name: String,
    pub path: PathBuf,
    pub replay: Option<Replay>,
    pub corrupt: bool,
}

impl ReplayFile {
    pub fn new(name: String, path: PathBuf) -> Self {
        ReplayFile {
            name,
            path,
            replay: None,
            corrupt: false,
        }
    }

    pub fn parse_replay(&mut self) -> Result<(), ParseError> {
        /* Attempt to read the entire file into a buffer */
        let buffer = fs::read(self.path.as_path()).map_err(|e| {
            ParseError::CorruptReplay(
                format!("Failed to read file: {}", e),
                Box::new(ParseError::ZeroSize) // Using ZeroSize as a placeholder
            )
        })?;

        /* Attempt to parse the replay file from the buffer */
        match boxcars::ParserBuilder::new(&buffer)
            .must_parse_network_data()
            .parse()
        {
            Ok(replay) => {
                self.replay = Some(replay);
                Ok(())
            }
            Err(e) => {
                self.corrupt = true;
                Err(e)
            }
        }
    }

    pub fn is_corrupt(&self) -> bool {
        self.corrupt
    }

    pub fn get_replay(&self) -> Option<&Replay> {
        self.replay.as_ref()
    }

    pub fn get_duration(&self) -> Option<f32> {
        self.replay.as_ref().and_then(|replay| {
            let properties: HashMap<_, _> = replay.properties
                .iter()
                .map(|(k, v)| (k.clone(), v))
                .collect();
    
            let record_fps = properties.get("RecordFPS").and_then(|prop| {
                if let HeaderProp::Float(fps) = prop {
                    Some(*fps)
                } else {
                    None
                }
            });
    
            let num_frames = properties.get("NumFrames").and_then(|prop| {
                if let HeaderProp::Int(frames) = prop {
                    Some(*frames)
                } else {
                    None
                }
            });
    
            match (record_fps, num_frames) {
                (Some(fps), Some(frames)) => {
                    let duration = frames as f32 / fps;
                    Some(duration)
                }
                _ => None,
            }
        })
    }
}
