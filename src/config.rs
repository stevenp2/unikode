use std::path::{Path, PathBuf};
use std::fs;
use std::io::{self, BufRead};
use structopt::StructOpt;

use crate::tools::PathMode::*;
use crate::tools::PathMode;
use crate::constants::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineNumberMode {
    Relative,
    Absolute,
}

impl std::str::FromStr for LineNumberMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "relative" | "rel" => Ok(LineNumberMode::Relative),
            "absolute" | "abs" => Ok(LineNumberMode::Absolute),
            _ => Err(format!("Invalid line mode: '{}'", s)),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Symbols {
    pub n: char,
    pub s: char,
    pub w: char,
    pub e: char,
    pub dash: char,
    pub pipe: char,
    pub diag: char,
    pub diag2: char,
    pub gaid: char,
    pub gaid2: char,
    pub plus: char,
    pub curs: char,
    pub brcorn: char,
    pub blcorn: char,
    pub trcorn: char,
    pub tlcorn: char,
    pub vline: char,
    pub hline: char,
    pub lhinter: char,
    pub rhinter: char,
    pub bvinter: char,
    pub tvinter: char,
    pub cinter: char,
    pub ubox: char,
}

impl Default for Symbols {
    fn default() -> Self {
        Self {
            n: N,
            s: S,
            w: W,
            e: E,
            dash: DASH,
            pipe: PIPE,
            diag: DIAG,
            diag2: DIAG2,
            gaid: GAID,
            gaid2: GAID2,
            plus: PLUS,
            curs: CURS,
            brcorn: BRCORN,
            blcorn: BLCORN,
            trcorn: TRCORN,
            tlcorn: TLCORN,
            vline: VLINE,
            hline: HLINE,
            lhinter: LHINTER,
            rhinter: RHINTER,
            bvinter: BVINTER,
            tvinter: TVINTER,
            cinter: CINTER,
            ubox: UBOX,
        }
    }
}

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

    /// Line number mode (relative or absolute).
    #[structopt(long)]
    pub line_mode: Option<LineNumberMode>,

    /// Move cursor to start of box after drawing (Box Mode).
    #[structopt(long)]
    pub box_cursor_start: bool,

    /// Show the current editor mode in the modeline.
    #[structopt(long)]
    pub show_mode: bool,

    /// Background color (hex or "transparent").
    #[structopt(long)]
    pub background: Option<String>,

    /// Normal text color (hex).
    #[structopt(long)]
    pub color_normal: Option<String>,

    /// Dirty/Unsaved text foreground color (hex).
    #[structopt(long)]
    pub color_dirty: Option<String>,

    /// Dirty/Unsaved text background color (hex).
    #[structopt(long)]
    pub color_dirty_bg: Option<String>,

    /// Cursor foreground color (hex).
    #[structopt(long)]
    pub color_cursor_fg: Option<String>,

    /// Cursor background color (hex).
    #[structopt(long)]
    pub color_cursor_bg: Option<String>,

    /// Selection foreground color (hex).
    #[structopt(long)]
    pub color_selection_fg: Option<String>,

    /// Selection background color (hex).
    #[structopt(long)]
    pub color_selection_bg: Option<String>,

    /// UI text color (hex) - used for line numbers and menu bar.
    #[structopt(long)]
    pub color_ui: Option<String>,

    /// Active UI text color (hex) - used for current line number and highlights.
    #[structopt(long)]
    pub color_ui_active: Option<String>,

    /// Symbols used for drawing.
    #[structopt(skip = Symbols::default())]
    pub symbols: Symbols,

    /// Text file to operate on.
    #[structopt(name = "FILE")]
    pub file: Option<PathBuf>,
}

pub fn parse_color(s: &str) -> Option<cursive::theme::Color> {
    use cursive::theme::Color;
    if s == "transparent" {
        return Some(Color::TerminalDefault);
    }
    if s.starts_with('#') && s.len() == 7 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&s[1..3], 16),
            u8::from_str_radix(&s[3..5], 16),
            u8::from_str_radix(&s[5..7], 16),
        ) {
            return Some(Color::Rgb(r, g, b));
        }
    }
    None
}

impl Options {
    pub fn cycle_path_mode(&mut self) {
        self.path_mode = match self.path_mode {
            Routed => Snap90,
            Snap90 => Routed,
        };
    }

    pub fn resolve_config(&mut self) {
        // Try to load from config file
        let config_paths = vec![
            PathBuf::from("unikode.conf"),
            std::env::var("HOME").map(|h| Path::new(&h).join(".unikoderc")).unwrap_or_else(|_| PathBuf::from(".unikoderc")),
            std::env::var("HOME").map(|h| Path::new(&h).join(".config/unikode/config")).unwrap_or_else(|_| PathBuf::from("")),
        ];

        for path in config_paths {
            if path.exists() {
                if let Ok(file) = fs::File::open(&path) {
                    let reader = io::BufReader::new(file);
                    for line in reader.lines() {
                        if let Ok(l) = line {
                            let parts: Vec<&str> = l.splitn(2, '=').map(|s| s.trim()).collect();
                            if parts.len() == 2 {
                                match parts[0] {
                                    "line_mode" => {
                                        if self.line_mode.is_none() {
                                            if let Ok(mode) = parts[1].parse() {
                                                self.line_mode = Some(mode);
                                            }
                                        }
                                    }
                                    "box_cursor_start" => {
                                        if let Ok(val) = parts[1].parse() {
                                            self.box_cursor_start = val;
                                        }
                                    }
                                    "show_mode" => {
                                        if let Ok(val) = parts[1].parse() {
                                            self.show_mode = val;
                                        }
                                    }
                                    "background" => {
                                        if self.background.is_none() {
                                            self.background = Some(parts[1].to_string());
                                        }
                                    }
                                    "color_normal" => {
                                        if self.color_normal.is_none() {
                                            self.color_normal = Some(parts[1].to_string());
                                        }
                                    }
                                    "color_dirty" => {
                                        if self.color_dirty.is_none() {
                                            self.color_dirty = Some(parts[1].to_string());
                                        }
                                    }
                                    "color_dirty_bg" => {
                                        if self.color_dirty_bg.is_none() {
                                            self.color_dirty_bg = Some(parts[1].to_string());
                                        }
                                    }
                                    "color_cursor_fg" => {
                                        if self.color_cursor_fg.is_none() {
                                            self.color_cursor_fg = Some(parts[1].to_string());
                                        }
                                    }
                                    "color_cursor_bg" => {
                                        if self.color_cursor_bg.is_none() {
                                            self.color_cursor_bg = Some(parts[1].to_string());
                                        }
                                    }
                                    "color_selection_fg" => {
                                        if self.color_selection_fg.is_none() {
                                            self.color_selection_fg = Some(parts[1].to_string());
                                        }
                                    }
                                    "color_selection_bg" => {
                                        if self.color_selection_bg.is_none() {
                                            self.color_selection_bg = Some(parts[1].to_string());
                                        }
                                    }
                                    "color_ui" | "color_linenum" => {
                                        if self.color_ui.is_none() {
                                            self.color_ui = Some(parts[1].to_string());
                                        }
                                    }
                                    "color_ui_active" | "color_linenum_active" => {
                                        if self.color_ui_active.is_none() {
                                            self.color_ui_active = Some(parts[1].to_string());
                                        }
                                    }
                                    "symbol_n" => if let Some(c) = parts[1].chars().next() { self.symbols.n = c; }
                                    "symbol_s" => if let Some(c) = parts[1].chars().next() { self.symbols.s = c; }
                                    "symbol_w" => if let Some(c) = parts[1].chars().next() { self.symbols.w = c; }
                                    "symbol_e" => if let Some(c) = parts[1].chars().next() { self.symbols.e = c; }
                                    "symbol_dash" => if let Some(c) = parts[1].chars().next() { self.symbols.dash = c; }
                                    "symbol_pipe" => if let Some(c) = parts[1].chars().next() { self.symbols.pipe = c; }
                                    "symbol_diag" => if let Some(c) = parts[1].chars().next() { self.symbols.diag = c; }
                                    "symbol_diag2" => if let Some(c) = parts[1].chars().next() { self.symbols.diag2 = c; }
                                    "symbol_gaid" => if let Some(c) = parts[1].chars().next() { self.symbols.gaid = c; }
                                    "symbol_gaid2" => if let Some(c) = parts[1].chars().next() { self.symbols.gaid2 = c; }
                                    "symbol_plus" => if let Some(c) = parts[1].chars().next() { self.symbols.plus = c; }
                                    "symbol_curs" => if let Some(c) = parts[1].chars().next() { self.symbols.curs = c; }
                                    "symbol_brcorn" => if let Some(c) = parts[1].chars().next() { self.symbols.brcorn = c; }
                                    "symbol_blcorn" => if let Some(c) = parts[1].chars().next() { self.symbols.blcorn = c; }
                                    "symbol_trcorn" => if let Some(c) = parts[1].chars().next() { self.symbols.trcorn = c; }
                                    "symbol_tlcorn" => if let Some(c) = parts[1].chars().next() { self.symbols.tlcorn = c; }
                                    "symbol_vline" => if let Some(c) = parts[1].chars().next() { self.symbols.vline = c; }
                                    "symbol_hline" => if let Some(c) = parts[1].chars().next() { self.symbols.hline = c; }
                                    "symbol_lhinter" => if let Some(c) = parts[1].chars().next() { self.symbols.lhinter = c; }
                                    "symbol_rhinter" => if let Some(c) = parts[1].chars().next() { self.symbols.rhinter = c; }
                                    "symbol_bvinter" => if let Some(c) = parts[1].chars().next() { self.symbols.bvinter = c; }
                                    "symbol_tvinter" => if let Some(c) = parts[1].chars().next() { self.symbols.tvinter = c; }
                                    "symbol_cinter" => if let Some(c) = parts[1].chars().next() { self.symbols.cinter = c; }
                                    "symbol_ubox" => if let Some(c) = parts[1].chars().next() { self.symbols.ubox = c; }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        // Default if still not set
        if self.line_mode.is_none() {
            self.line_mode = Some(LineNumberMode::Relative);
        }
    }
}
