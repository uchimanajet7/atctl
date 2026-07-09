pub fn mask_sensitive_values(input: &str) -> String {
    let input = mask_cgauth_values(input);
    let input = mask_qccid_values(&input);
    let mut output = String::with_capacity(input.len());
    let mut digits = String::new();

    for ch in input.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            push_masked_digits(&mut output, &digits);
            digits.clear();
            output.push(ch);
        }
    }

    push_masked_digits(&mut output, &digits);
    output
}

fn mask_cgauth_values(input: &str) -> String {
    input
        .split_inclusive('\n')
        .map(mask_cgauth_line)
        .collect::<String>()
}

fn mask_cgauth_line(line: &str) -> String {
    if let Some(params_start) = cgauth_params_start(line) {
        return mask_cgauth_params(line, params_start);
    }

    line.to_owned()
}

fn cgauth_params_start(line: &str) -> Option<usize> {
    if let Some(prefix_end) = find_ascii_case_insensitive(line, "AT+CGAUTH=") {
        return Some(skip_ascii_whitespace(line, prefix_end));
    }

    find_ascii_case_insensitive(line, "+CGAUTH:")
        .map(|prefix_end| skip_ascii_whitespace(line, prefix_end))
}

fn skip_ascii_whitespace(value: &str, start: usize) -> usize {
    start
        + value[start..]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .map(char::len_utf8)
            .sum::<usize>()
}

fn mask_cgauth_params(line: &str, params_start: usize) -> String {
    let params = &line[params_start..];
    let mut fields = Vec::new();
    let mut field_start = 0;
    let mut in_quotes = false;

    for (index, ch) in params.char_indices() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                fields.push(&params[field_start..index]);
                field_start = index + ch.len_utf8();
            }
            _ => {}
        }
    }

    fields.push(&params[field_start..]);

    let should_mask_credentials = cgauth_fields_have_credentials(&fields);
    let mut output = String::with_capacity(line.len());
    output.push_str(&line[..params_start]);

    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        push_cgauth_field(&mut output, field, index, should_mask_credentials);
    }

    output
}

fn cgauth_fields_have_credentials(fields: &[&str]) -> bool {
    fields.len() >= 4 && is_ascii_integer_field(fields[0]) && is_ascii_integer_field(fields[1])
}

fn is_ascii_integer_field(field: &str) -> bool {
    let trimmed = field.trim();
    !trimmed.is_empty() && trimmed.chars().all(|ch| ch.is_ascii_digit())
}

fn push_cgauth_field(output: &mut String, field: &str, index: usize, should_mask: bool) {
    if should_mask && matches!(index, 2 | 3) {
        push_masked_auth_field(output, field);
    } else {
        output.push_str(field);
    }
}

fn push_masked_auth_field(output: &mut String, field: &str) {
    let leading_len = field
        .chars()
        .take_while(|ch| ch.is_ascii_whitespace())
        .map(char::len_utf8)
        .sum::<usize>();
    let trailing_len = field[leading_len..]
        .chars()
        .rev()
        .take_while(|ch| ch.is_ascii_whitespace())
        .map(char::len_utf8)
        .sum::<usize>();
    let core_end = field.len() - trailing_len;
    let core = &field[leading_len..core_end];

    output.push_str(&field[..leading_len]);
    if core.len() >= 2 && core.starts_with('"') && core.ends_with('"') {
        output.push('"');
        output.push_str(&mask_identifier(&core[1..core.len() - 1]));
        output.push('"');
    } else {
        output.push_str(&mask_identifier(core));
    }
    output.push_str(&field[core_end..]);
}

fn mask_qccid_values(input: &str) -> String {
    input
        .split_inclusive('\n')
        .map(mask_qccid_line)
        .collect::<String>()
}

fn mask_qccid_line(line: &str) -> String {
    let Some(prefix_end) = find_ascii_case_insensitive(line, "+QCCID:") else {
        return line.to_owned();
    };

    let value_start = prefix_end
        + line[prefix_end..]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .map(char::len_utf8)
            .sum::<usize>();
    let value_len = line[value_start..]
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric())
        .map(char::len_utf8)
        .sum::<usize>();

    if value_len == 0 {
        return line.to_owned();
    }

    let value_end = value_start + value_len;
    let mut output = String::with_capacity(line.len());
    output.push_str(&line[..value_start]);
    push_masked_identifier(&mut output, &line[value_start..value_end]);
    output.push_str(&line[value_end..]);
    output
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|candidate| candidate.eq_ignore_ascii_case(needle.as_bytes()))
        .map(|start| start + needle.len())
}

pub fn mask_identifier(input: &str) -> String {
    let chars = input.chars().collect::<Vec<_>>();
    match chars.len() {
        0 => String::new(),
        1..=4 => "*".repeat(chars.len()),
        5..=8 => {
            let prefix = chars.iter().take(2).collect::<String>();
            format!("{prefix}{}", "*".repeat(chars.len() - 2))
        }
        len => {
            let prefix = chars.iter().take(4).collect::<String>();
            let suffix = chars.iter().skip(len - 2).collect::<String>();
            format!("{prefix}{}{suffix}", "*".repeat(len - 6))
        }
    }
}

fn push_masked_digits(output: &mut String, digits: &str) {
    if digits.is_empty() {
        return;
    }

    if digits.len() >= 12 {
        push_masked_identifier(output, digits);
    } else {
        output.push_str(digits);
    }
}

fn push_masked_identifier(output: &mut String, value: &str) {
    let visible_prefix = value.len().min(8);
    output.push_str(&value[..visible_prefix]);
    output.push_str(&"*".repeat(value.len() - visible_prefix));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_long_identifiers() {
        assert_eq!(
            mask_sensitive_values("+QCCID: 89811000123456789012"),
            "+QCCID: 89811000************"
        );
    }

    #[test]
    fn masks_qccid_padding_f_as_part_of_sensitive_value() {
        assert_eq!(
            mask_sensitive_values("+QCCID: 8942310020003626445F"),
            "+QCCID: 89423100************"
        );
    }

    #[test]
    fn masks_cgauth_command_credentials() {
        assert_eq!(
            mask_sensitive_values("AT+CGAUTH=1,2,\"sora\",\"sora\""),
            "AT+CGAUTH=1,2,\"****\",\"****\""
        );
    }

    #[test]
    fn masks_cgauth_response_credentials() {
        assert_eq!(
            mask_sensitive_values("+CGAUTH: 1,2,\"custom-user\",\"custom-password\"\nOK"),
            "+CGAUTH: 1,2,\"cust*****er\",\"cust*********rd\"\nOK"
        );
    }

    #[test]
    fn leaves_cgauth_capability_test_plain() {
        assert_eq!(mask_sensitive_values("AT+CGAUTH=?"), "AT+CGAUTH=?");
    }

    #[test]
    fn leaves_cgauth_capability_response_plain() {
        assert_eq!(
            mask_sensitive_values("+CGAUTH: (1-16),(0-3),(0-64),(0-64)"),
            "+CGAUTH: (1-16),(0-3),(0-64),(0-64)"
        );
    }

    #[test]
    fn leaves_short_numbers_plain() {
        assert_eq!(mask_sensitive_values("+CSQ: 18,99"), "+CSQ: 18,99");
    }

    #[test]
    fn masks_identifier_values() {
        assert_eq!(mask_identifier("ABCDEF123456"), "ABCD******56");
        assert_eq!(mask_identifier("1234"), "****");
    }
}
