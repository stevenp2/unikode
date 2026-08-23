use cursive::theme::Color;

pub(crate) fn parse_color(value: &str) -> Option<Color> {
    if value == "transparent" {
        return Some(Color::TerminalDefault);
    }
    if value.starts_with('#') && value.len() == 7 {
        if let (Ok(red), Ok(green), Ok(blue)) = (
            u8::from_str_radix(&value[1..3], 16),
            u8::from_str_radix(&value[3..5], 16),
            u8::from_str_radix(&value[5..7], 16),
        ) {
            return Some(Color::Rgb(red, green, blue));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_terminal_and_rgb_colors() {
        assert_eq!(parse_color("transparent"), Some(Color::TerminalDefault));
        assert_eq!(parse_color("#12aBef"), Some(Color::Rgb(0x12, 0xab, 0xef)));
        assert_eq!(parse_color("blue"), None);
    }
}
