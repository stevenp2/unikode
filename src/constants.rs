use cursive::{
    view::Margins,
    event::EventResult,
};

use std::f64::consts::SQRT_2;

pub(crate) const EDITOR_ID: &str = "editor";
pub(crate) const S90: &str = "Snap90";
pub(crate) const RTD: &str = "Routed";

pub(crate) const N: char = '▲';
pub(crate) const S: char = '▼';
pub(crate) const W: char = '◀';
pub(crate) const E: char = '▶';

pub(crate) const S_N: (isize, isize) = (0, -1);
pub(crate) const S_E: (isize, isize) = (1, 0);
pub(crate) const S_S: (isize, isize) = (0, 1);
pub(crate) const S_W: (isize, isize) = (-1, 0);

/// Cost to move one step on the cardinal plane.
pub(crate) const D: f64 = 1.0;
/// Cost to move one step on the diagonal plane.
pub(crate) const D2: f64 = SQRT_2;

pub(crate) const SP: char = ' ';
pub(crate) const DASH: char = '-';
pub(crate) const PIPE: char = '|';
pub(crate) const DIAG: char = '/';
pub(crate) const DIAG2: char = '╱';
pub(crate) const GAID: char = '\\';
pub(crate) const GAID2: char = '╲';
pub(crate) const PLUS: char = '+';
pub(crate) const CURS: char = '_';

// box drawing
pub(crate) const BRCORN: char = '┘';
pub(crate) const BLCORN: char = '└';
pub(crate) const TRCORN: char = '┐';
pub(crate) const TLCORN: char = '┌';
pub(crate) const VLINE: char = '│';
pub(crate) const HLINE: char = '─';

pub(crate) const LHINTER: char = '├';
pub(crate) const RHINTER: char = '┤';
pub(crate) const BVINTER: char = '┴';
pub(crate) const TVINTER: char = '┬';
pub(crate) const CINTER: char = '┼';
pub(crate) const UBOX: char = '□';

pub(crate) const CONSUMED: Option<EventResult> = Some(EventResult::Consumed(None));

pub(crate) const NO_MARGIN: Margins = Margins {
    left: 0,
    right: 0,
    top: 0,
    bottom: 0,
};

pub(crate) const POPUP_ID: &str = "generic_popup";

pub(crate) const INPUT_ID: &str = "generic_input";

pub(crate) const KEY_UNDO: char = 'u';
pub(crate) const KEY_SAVE: char = 'w';
pub(crate) const KEY_SAVE_AS: char = 'S';
pub(crate) const KEY_CLIP: char = 'c';
pub(crate) const KEY_CLIP_PREFIX: char = 'C';
pub(crate) const KEY_NEW: char = 'n';
pub(crate) const KEY_OPEN: char = 'o';
pub(crate) const KEY_QUIT: char = 'q';
pub(crate) const KEY_DEBUG: char = '`';
pub(crate) const KEY_CYCLE_PATH: char = 'p';
pub(crate) const KEY_TRIM_MARGINS: char = 'T';
pub(crate) const KEY_HELP: char = '?';

pub(crate) const KEY_MOVE_LEFT: char = 'h';
pub(crate) const KEY_MOVE_DOWN: char = 'j';
pub(crate) const KEY_MOVE_UP: char = 'k';
pub(crate) const KEY_MOVE_RIGHT: char = 'l';

pub(crate) const KEY_MOVE_LINE_START: char = '0';
pub(crate) const KEY_MOVE_FIRST_NON_WS: char = '^';
pub(crate) const KEY_MOVE_LAST_NON_WS: char = '$';

pub(crate) const KEY_TOOL_BOX: char = 'b';
pub(crate) const KEY_TOOL_LINE: char = 'L';
pub(crate) const KEY_TOOL_ARROW: char = 'a';
pub(crate) const KEY_TOOL_TEXT: char = 't';
pub(crate) const KEY_TOOL_SELECT: char = 's';
pub(crate) const KEY_TOOL_ERASE: char = 'e';
pub(crate) const KEY_TOOL_MOVE: char = 'm';

pub(crate) const GUTTER_WIDTH: usize = 5;

// Default Colours
pub(crate) const DEFAULT_BACKGROUND: &str = "#212121";
pub(crate) const DEFAULT_COLOR_NORMAL: &str = "#ffffff";
pub(crate) const DEFAULT_COLOR_DIRTY: &str = "#ffffff";
pub(crate) const DEFAULT_COLOR_DIRTY_BG: &str = "#316AC5";
pub(crate) const DEFAULT_COLOR_CURSOR_FG: &str = "#ffffff";
pub(crate) const DEFAULT_COLOR_CURSOR_BG: &str = "#316AC5";
pub(crate) const DEFAULT_COLOR_SELECTION_FG: &str = "#ffffff";
pub(crate) const DEFAULT_COLOR_SELECTION_BG: &str = "#316AC5";
pub(crate) const DEFAULT_COLOR_UI: &str = "#ffffff";
pub(crate) const DEFAULT_COLOR_UI_ACTIVE: &str = "#ffff00";
