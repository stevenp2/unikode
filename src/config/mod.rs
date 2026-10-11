use clap::{Parser, ValueEnum};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

use self::defaults::*;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PathMode {
    #[default]
    Snap90,
    Routed,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum RenderMode {
    /// orthogonal paths drawn with box characters
    #[default]
    Classic,
    /// 2x4 braille raster with cardinal arrowheads
    Braille,
}

impl std::str::FromStr for RenderMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "classic" => Ok(Self::Classic),
            "braille" => Ok(Self::Braille),
            _ => Err(format!("Invalid render mode: '{value}'")),
        }
    }
}

mod defaults;

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
            _ => Err(format!("Invalid line mode: '{s}'")),
        }
    }
}

fn first_char(s: &str, default: char) -> char {
    s.chars().next().unwrap_or(default)
}

macro_rules! char_config {
    (
        $(#[$runtime_meta:meta])*
        pub $runtime:ident => $toml:ident {
            $( $field:ident : $default:expr ),* $(,)?
        }
    ) => {
        $(#[$runtime_meta])*
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $runtime {
            $(pub $field: char,)*
        }

        impl Default for $runtime {
            fn default() -> Self {
                Self { $($field: $default,)* }
            }
        }

        #[derive(Debug, Deserialize)]
        #[serde(default)]
        struct $toml {
            $($field: String,)*
        }

        impl Default for $toml {
            fn default() -> Self {
                Self { $($field: $default.to_string(),)* }
            }
        }

        impl From<$toml> for $runtime {
            fn from(cfg: $toml) -> Self {
                Self { $($field: first_char(&cfg.$field, $default),)* }
            }
        }
    };
}

macro_rules! str_config {
    (
        $name:ident {
            $( $field:ident : $default:expr ),* $(,)?
        }
    ) => {
        #[derive(Debug, Deserialize)]
        #[serde(default)]
        struct $name {
            $($field: String,)*
        }

        impl Default for $name {
            fn default() -> Self {
                Self { $($field: $default.to_string(),)* }
            }
        }
    };
}

char_config! {
    pub Symbols => TomlSymbolConfig {
        n:       N,
        s:       S,
        w:       W,
        e:       E,
        dash:    DASH,
        pipe:    PIPE,
        diag:    DIAG,
        diag2:   DIAG2,
        gaid:    GAID,
        gaid2:   GAID2,
        plus:    PLUS,
        curs:    CURS,
        brcorn:  BRCORN,
        blcorn:  BLCORN,
        trcorn:  TRCORN,
        tlcorn:  TLCORN,
        vline:   VLINE,
        hline:   HLINE,
        lhinter: LHINTER,
        rhinter: RHINTER,
        bvinter: BVINTER,
        tvinter: TVINTER,
        cinter:  CINTER,
        ubox:    UBOX,
    }
}

char_config! {
    pub KeyBindings => TomlKeyConfig {
        undo:             KEY_UNDO,
        save:             KEY_SAVE,
        save_as:          KEY_SAVE_AS,
        clip:             KEY_CLIP,
        clip_prefix:      KEY_CLIP_PREFIX,
        new:              KEY_NEW,
        open:             KEY_OPEN,
        quit:             KEY_QUIT,
        debug:            KEY_DEBUG,
        cycle_path:       KEY_CYCLE_PATH,
        trim_margins:     KEY_TRIM_MARGINS,
        help:             KEY_HELP,
        move_left:        KEY_MOVE_LEFT,
        move_down:        KEY_MOVE_DOWN,
        move_up:          KEY_MOVE_UP,
        move_right:       KEY_MOVE_RIGHT,
        move_line_start:  KEY_MOVE_LINE_START,
        move_first_non_ws: KEY_MOVE_FIRST_NON_WS,
        move_last_non_ws: KEY_MOVE_LAST_NON_WS,
        tool_box:         KEY_TOOL_BOX,
        tool_line:        KEY_TOOL_LINE,
        tool_arrow:       KEY_TOOL_ARROW,
        tool_text:        KEY_TOOL_TEXT,
        tool_select:      KEY_TOOL_SELECT,
        tool_erase:       KEY_TOOL_ERASE,
        tool_move:        KEY_TOOL_MOVE,
    }
}

impl KeyBindings {
    pub fn is_movement(&self, c: char) -> bool {
        c == self.move_left || c == self.move_down || c == self.move_up || c == self.move_right
    }

    pub fn is_global(&self, c: char) -> bool {
        c == self.undo
            || c == self.save
            || c == self.save_as
            || c == self.clip
            || c == self.clip_prefix
            || c == self.new
            || c == self.open
            || c == self.quit
            || c == self.debug
            || c == self.cycle_path
            || c == self.trim_margins
            || c == self.help
    }
}

str_config! {
    TomlColorConfig {
        background:   BACKGROUND,
        normal:       COLOR_NORMAL,
        dirty:        COLOR_DIRTY,
        dirty_bg:     COLOR_DIRTY_BG,
        cursor_fg:    COLOR_CURSOR_FG,
        cursor_bg:    COLOR_CURSOR_BG,
        selection_fg: COLOR_SELECTION_FG,
        selection_bg: COLOR_SELECTION_BG,
        ui:           COLOR_UI,
        ui_active:    COLOR_UI_ACTIVE,
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct TomlConfig {
    editor: TomlEditorConfig,
    colors: TomlColorConfig,
    symbols: TomlSymbolConfig,
    keys: TomlKeyConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct TomlEditorConfig {
    line_mode: String,
    show_mode: bool,
    box_cursor_start: bool,
    keep_trailing_ws: bool,
    strip_margin_ws: bool,
    path_mode: String,
    render_mode: String,
    gutter_width: usize,
    scroll: TomlScrollConfig,
}

impl Default for TomlEditorConfig {
    fn default() -> Self {
        Self {
            line_mode: "relative".to_string(),
            show_mode: false,
            box_cursor_start: false,
            keep_trailing_ws: false,
            strip_margin_ws: false,
            path_mode: "snap90".to_string(),
            render_mode: "classic".to_string(),
            gutter_width: GUTTER_WIDTH,
            scroll: TomlScrollConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct TomlScrollConfig {
    mouse_drag_step: usize,
    cursor_step: usize,
}

impl Default for TomlScrollConfig {
    fn default() -> Self {
        Self {
            mouse_drag_step: SCROLL_MOUSE_DRAG_STEP,
            cursor_step: SCROLL_CURSOR_STEP,
        }
    }
}

#[derive(Clone, Debug, Parser)]
#[command(author = "stevenp2 <sphung.808@gmail.com>.", version)]
pub struct Options {
    #[arg(skip = PathMode::Snap90)]
    pub path_mode: PathMode,

    #[arg(skip = RenderMode::Classic)]
    pub render_mode: RenderMode,

    #[arg(long = "render-mode", value_enum)]
    render_mode_override: Option<RenderMode>,

    #[arg(short, long)]
    pub keep_trailing_ws: bool,

    #[arg(long, conflicts_with = "keep_trailing_ws")]
    no_keep_trailing_ws: bool,

    #[arg(short, long)]
    pub strip_margin_ws: bool,

    #[arg(long, conflicts_with = "strip_margin_ws")]
    no_strip_margin_ws: bool,

    #[arg(long)]
    pub line_mode: Option<LineNumberMode>,

    #[arg(long)]
    pub box_cursor_start: bool,

    #[arg(long, conflicts_with = "box_cursor_start")]
    no_box_cursor_start: bool,

    #[arg(long)]
    pub show_mode: bool,

    #[arg(long, conflicts_with = "show_mode")]
    no_show_mode: bool,

    #[arg(long)]
    pub background: Option<String>,

    #[arg(long)]
    pub color_normal: Option<String>,

    #[arg(long)]
    pub color_dirty: Option<String>,

    #[arg(long)]
    pub color_dirty_bg: Option<String>,

    #[arg(long)]
    pub color_cursor_fg: Option<String>,

    #[arg(long)]
    pub color_cursor_bg: Option<String>,

    #[arg(long)]
    pub color_selection_fg: Option<String>,

    #[arg(long)]
    pub color_selection_bg: Option<String>,

    #[arg(long)]
    pub color_ui: Option<String>,

    #[arg(long)]
    pub color_ui_active: Option<String>,

    #[arg(skip = Symbols::default())]
    pub symbols: Symbols,

    #[arg(skip = KeyBindings::default())]
    pub keys: KeyBindings,

    #[arg(skip = GUTTER_WIDTH)]
    pub gutter_width: usize,

    #[arg(skip = SCROLL_MOUSE_DRAG_STEP)]
    pub scroll_mouse_drag_step: usize,

    #[arg(skip = SCROLL_CURSOR_STEP)]
    pub scroll_cursor_step: usize,

    #[arg(value_name = "FILE")]
    pub file: Option<PathBuf>,
}

impl Options {
    pub fn with_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.file = Some(file.into());
        self
    }

    pub fn with_path_mode(mut self, path_mode: PathMode) -> Self {
        self.path_mode = path_mode;
        self
    }

    pub fn with_render_mode(mut self, render_mode: RenderMode) -> Self {
        self.render_mode = render_mode;
        self
    }

    /// use snap90 for classic because box characters cannot join diagonals
    pub fn effective_path_mode(&self) -> PathMode {
        match self.render_mode {
            RenderMode::Classic => PathMode::Snap90,
            RenderMode::Braille => self.path_mode,
        }
    }

    /// cycle braille paths, returning false in classic mode
    pub fn cycle_path_mode(&mut self) -> bool {
        if self.render_mode == RenderMode::Classic {
            return false;
        }
        self.path_mode = match self.path_mode {
            PathMode::Routed => PathMode::Snap90,
            PathMode::Snap90 => PathMode::Routed,
        };
        true
    }

    pub fn resolve_config(&mut self) {
        let config_paths = [
            dirs::config_dir().map(|d| d.join("unikode").join("unikode.toml")),
            Some(PathBuf::from("unikode.toml")),
        ];

        self.resolve_config_from_paths(config_paths.into_iter().flatten());
    }

    /// load the first existing path, using defaults if reading or parsing fails
    /// warn on parse errors
    pub fn resolve_config_from_paths<I, P>(&mut self, config_paths: I)
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        let config_paths: Vec<PathBuf> = config_paths.into_iter().map(Into::into).collect();

        let toml_config: TomlConfig = config_paths
            .iter()
            .find(|p| p.exists())
            .and_then(|p| {
                let content = fs::read_to_string(p).ok()?;
                match toml::from_str(&content) {
                    Ok(cfg) => Some(cfg),
                    Err(e) => {
                        eprintln!("Warning: Failed to parse config at {p:?}: {e}");
                        None
                    }
                }
            })
            .unwrap_or_default();

        self.apply_toml(toml_config);
    }

    fn apply_toml(&mut self, cfg: TomlConfig) {
        if self.line_mode.is_none() {
            self.line_mode = Some(
                cfg.editor
                    .line_mode
                    .parse()
                    .unwrap_or(LineNumberMode::Relative),
            );
        }

        if self.no_show_mode {
            self.show_mode = false;
        } else if !self.show_mode {
            self.show_mode = cfg.editor.show_mode;
        }
        if self.no_box_cursor_start {
            self.box_cursor_start = false;
        } else if !self.box_cursor_start {
            self.box_cursor_start = cfg.editor.box_cursor_start;
        }
        if self.no_keep_trailing_ws {
            self.keep_trailing_ws = false;
        } else if !self.keep_trailing_ws {
            self.keep_trailing_ws = cfg.editor.keep_trailing_ws;
        }
        if self.no_strip_margin_ws {
            self.strip_margin_ws = false;
        } else if !self.strip_margin_ws {
            self.strip_margin_ws = cfg.editor.strip_margin_ws;
        }

        let configured_render_mode = cfg
            .editor
            .render_mode
            .parse()
            .unwrap_or(RenderMode::Classic);
        self.render_mode = self.render_mode_override.unwrap_or(configured_render_mode);

        self.path_mode = match cfg.editor.path_mode.to_lowercase().as_str() {
            "routed" => PathMode::Routed,
            _ => PathMode::Snap90,
        };
        if self.render_mode == RenderMode::Classic {
            self.path_mode = PathMode::Snap90;
        }

        self.gutter_width = cfg.editor.gutter_width;
        self.scroll_mouse_drag_step = cfg.editor.scroll.mouse_drag_step;
        self.scroll_cursor_step = cfg.editor.scroll.cursor_step;

        macro_rules! apply_color {
            ($opt:ident <- $cfg_field:ident) => {
                if self.$opt.is_none() {
                    self.$opt = Some(cfg.colors.$cfg_field);
                }
            };
        }
        apply_color!(background      <- background);
        apply_color!(color_normal     <- normal);
        apply_color!(color_dirty      <- dirty);
        apply_color!(color_dirty_bg   <- dirty_bg);
        apply_color!(color_cursor_fg  <- cursor_fg);
        apply_color!(color_cursor_bg  <- cursor_bg);
        apply_color!(color_selection_fg <- selection_fg);
        apply_color!(color_selection_bg <- selection_bg);
        apply_color!(color_ui         <- ui);
        apply_color!(color_ui_active  <- ui_active);

        self.symbols = cfg.symbols.into();
        self.keys = cfg.keys.into();
    }
}

impl Default for Options {
    fn default() -> Self {
        Self::try_parse_from(["unikode"]).expect("the built-in argument set must always be valid")
    }
}
