

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about)]
#[clap(arg_required_else_help = true)]
pub struct RustReplay {
    #[clap(subcommand)]
    pub subcommand: SubCommands,
}

#[derive(Subcommand)]
pub enum SubCommands {
    /// List all the replays in the save directory, and with some their metadata if verbose
    #[clap(visible_alias = "demos")]
    List {
        /// Show additional information about the replay
        #[clap(long, short)]
        verbose: bool,
        /// Output information in markdown format and alphabetical order
        ///
        /// Useful for creating modpack mod lists.
        /// Complements the verbose flag.
        #[clap(long, short, visible_alias = "md")]
        markdown: bool,
    },
}
