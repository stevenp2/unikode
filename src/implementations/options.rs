use std::path::PathBuf;
use structopt::StructOpt;

use crate::tools::PathMode::*;
use crate::tools::PathMode;

#[derive(Clone, Debug, StructOpt)]
#[structopt(
    author = "made with love by nytopop <ericizoita@gmail.com>.",
    help_message = "print help information",
    version_message = "print version information"
)]

pub(crate) struct Options {
    /// How paths are routed.
    #[structopt(skip = PathMode::Snap90)]
    pub path_mode: PathMode,

    /// Keep trailing whitespace (on save).
    #[structopt(short, long)]
    pub keep_trailing_ws: bool,

    /// Strip all margin whitespace (on save).
    #[structopt(short, long)]
    pub strip_margin_ws: bool,

    /// Text file to operate on.
    #[structopt(name = "FILE")]
    pub file: Option<PathBuf>,
}

impl Options {
    pub fn cycle_path_mode(&mut self) {
        self.path_mode = match self.path_mode {
            Routed => Snap90,
            Snap90 => Routed,
        };
    }
}
