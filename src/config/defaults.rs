// Editor settings
pub(crate) const GUTTER_WIDTH: usize = 5;
pub(crate) const SCROLL_MOUSE_DRAG_STEP: usize = 2;
pub(crate) const SCROLL_CURSOR_STEP: usize = 1;

// Default colours
pub(crate) const BACKGROUND: &str = "#212121";
pub(crate) const COLOR_NORMAL: &str = "#ffffff";
pub(crate) const COLOR_DIRTY: &str = "#ffffff";
pub(crate) const COLOR_DIRTY_BG: &str = "#316AC5";
pub(crate) const COLOR_CURSOR_FG: &str = "#ffffff";
pub(crate) const COLOR_CURSOR_BG: &str = "#316AC5";
pub(crate) const COLOR_SELECTION_FG: &str = "#ffffff";
pub(crate) const COLOR_SELECTION_BG: &str = "#316AC5";
pub(crate) const COLOR_UI: &str = "#ffffff";
pub(crate) const COLOR_UI_ACTIVE: &str = "#ffff00";

// Default symbols
pub(crate) const N: char = '▲';
pub(crate) const S: char = '▼';
pub(crate) const W: char = '◀';
pub(crate) const E: char = '▶';

pub(crate) const DASH: char = '-';
pub(crate) const PIPE: char = '|';
pub(crate) const DIAG: char = '/';
pub(crate) const DIAG2: char = '╱';
pub(crate) const GAID: char = '\\';
pub(crate) const GAID2: char = '╲';
pub(crate) const PLUS: char = '+';
pub(crate) const CURS: char = '_';

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

// Default key bindings

/// Commands
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

/// Movement
pub(crate) const KEY_MOVE_LEFT: char = 'h';
pub(crate) const KEY_MOVE_DOWN: char = 'j';
pub(crate) const KEY_MOVE_UP: char = 'k';
pub(crate) const KEY_MOVE_RIGHT: char = 'l';
pub(crate) const KEY_MOVE_LINE_START: char = '0';
pub(crate) const KEY_MOVE_FIRST_NON_WS: char = '^';
pub(crate) const KEY_MOVE_LAST_NON_WS: char = '$';

/// Tool switching
pub(crate) const KEY_TOOL_BOX: char = 'b';
pub(crate) const KEY_TOOL_LINE: char = 'L';
pub(crate) const KEY_TOOL_ARROW: char = 'a';
pub(crate) const KEY_TOOL_TEXT: char = 't';
pub(crate) const KEY_TOOL_SELECT: char = 's';
pub(crate) const KEY_TOOL_ERASE: char = 'e';
pub(crate) const KEY_TOOL_MOVE: char = 'm';
