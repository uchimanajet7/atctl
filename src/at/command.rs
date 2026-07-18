pub fn normalize_command(command: &str) -> String {
    command
        .trim()
        .trim_end_matches(['\r', '\n'])
        .trim()
        .to_ascii_uppercase()
}

pub fn command_with_terminator(command: &str) -> String {
    if command.ends_with('\r') || command.ends_with('\n') {
        command.to_string()
    } else {
        format!("{command}\r")
    }
}

pub fn has_embedded_command_framing(command: &str) -> bool {
    let command = command.trim_end_matches(['\r', '\n']);
    command.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_carriage_return_once() {
        assert_eq!(command_with_terminator("AT"), "AT\r");
        assert_eq!(command_with_terminator("AT\r"), "AT\r");
    }

    #[test]
    fn normalizes_for_classification() {
        assert_eq!(normalize_command(" at+cimi\r\n"), "AT+CIMI");
    }

    #[test]
    fn rejects_embedded_command_framing_but_allows_a_trailing_terminator() {
        assert!(!has_embedded_command_framing("AT+CSQ"));
        assert!(!has_embedded_command_framing("AT+CSQ\r"));
        assert!(has_embedded_command_framing("AT+CSQ\rAT+CFUN=1"));
        assert!(has_embedded_command_framing("AT+CMGS=1\u{1a}"));
    }
}
