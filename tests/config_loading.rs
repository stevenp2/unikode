mod common;

use std::fs;
use unikode::{
    config::{LineNumberMode, Options},
    PathMode, RenderMode,
};

use common::TestDir;

#[test]
fn explicit_config_paths_are_deterministic_and_first_existing_wins() {
    let dir = TestDir::new("config");
    let first = dir.join("first.toml");
    let second = dir.join("second.toml");
    fs::write(
        &first,
        "[editor]\nline_mode = \"absolute\"\nrender_mode = \"braille\"\npath_mode = \"routed\"\ngutter_width = 7\n[symbols]\nhline = \"=\"\n",
    )
    .unwrap();
    fs::write(&second, "[editor]\ngutter_width = 99\n").unwrap();

    let mut options = Options::default();
    options.resolve_config_from_paths([dir.join("missing.toml"), first, second]);

    assert_eq!(options.line_mode, Some(LineNumberMode::Absolute));
    assert_eq!(options.render_mode, RenderMode::Braille);
    assert_eq!(options.path_mode, PathMode::Routed);
    assert_eq!(options.gutter_width, 7);
    assert_eq!(options.symbols.hline, '=');
}

#[test]
fn classic_mode_disables_path_mode_cycling() {
    let mut options = Options::default().with_path_mode(PathMode::Routed);
    assert_eq!(options.render_mode, RenderMode::Classic);
    assert_eq!(options.effective_path_mode(), PathMode::Snap90);
    assert!(!options.cycle_path_mode());
    assert_eq!(options.path_mode, PathMode::Routed);

    options = options.with_render_mode(RenderMode::Braille);
    assert_eq!(options.effective_path_mode(), PathMode::Routed);
    assert!(options.cycle_path_mode());
    assert_eq!(options.path_mode, PathMode::Snap90);
}

#[test]
fn cli_render_mode_overrides_toml() {
    use clap::Parser;

    let dir = TestDir::new("render-mode-precedence");
    let config = dir.join("unikode.toml");
    fs::write(
        &config,
        "[editor]\nrender_mode = \"classic\"\npath_mode = \"routed\"\n",
    )
    .unwrap();

    let mut options = Options::try_parse_from(["unikode", "--render-mode", "braille"]).unwrap();
    options.resolve_config_from_paths([config]);

    assert_eq!(options.render_mode, RenderMode::Braille);
    assert_eq!(options.path_mode, PathMode::Routed);
}

#[test]
fn cli_values_take_precedence_over_toml_values() {
    use clap::Parser;

    let dir = TestDir::new("precedence");
    let config = dir.join("unikode.toml");
    fs::write(
        &config,
        "[editor]\nkeep_trailing_ws = false\nline_mode = \"relative\"\n",
    )
    .unwrap();

    let mut options =
        Options::try_parse_from(["unikode", "--keep-trailing-ws", "--line-mode", "absolute"])
            .unwrap();
    options.resolve_config_from_paths([config]);

    assert!(options.keep_trailing_ws);
    assert_eq!(options.line_mode, Some(LineNumberMode::Absolute));
}

#[test]
fn negative_cli_flags_can_override_true_toml_booleans() {
    use clap::Parser;

    let dir = TestDir::new("negative-precedence");
    let config = dir.join("unikode.toml");
    fs::write(
        &config,
        "[editor]\nkeep_trailing_ws = true\nshow_mode = true\n",
    )
    .unwrap();

    let mut options =
        Options::try_parse_from(["unikode", "--no-keep-trailing-ws", "--no-show-mode"]).unwrap();
    options.resolve_config_from_paths([config]);

    assert!(!options.keep_trailing_ws);
    assert!(!options.show_mode);
}
