pub fn mask_sensitive_values(input: &str) -> String {
    let input = mask_cgauth_values(input);
    let input = mask_qccid_values(&input);
    let input = mask_known_credential_fields(&input);
    let input = mask_known_phone_and_sms_fields(&input);
    mask_long_digit_runs(&input)
}

/// Masks a product-known phone/MSISDN value without relying on its length.
/// Protocol punctuation remains visible so the surrounding AT field is usable.
pub fn mask_phone_number(input: &str) -> String {
    input
        .chars()
        .map(|ch| if ch.is_ascii_digit() { '*' } else { ch })
        .collect()
}

/// Fully hides a value whose provenance declares it sensitive.
/// Separators remain visible, but no alphanumeric content is retained.
pub fn mask_declared_sensitive_value(input: &str) -> String {
    input
        .chars()
        .map(|ch| if ch.is_alphanumeric() { '*' } else { ch })
        .collect()
}

fn mask_long_digit_runs(input: &str) -> String {
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

fn mask_known_phone_and_sms_fields(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut mask_sms_body = false;
    let lines = input.split_inclusive('\n').collect::<Vec<_>>();
    let last_content_index = lines
        .iter()
        .rposition(|line| !split_line_ending(line).0.trim().is_empty());

    for (line_index, line) in lines.into_iter().enumerate() {
        let (content, ending) = split_line_ending(line);
        if let Some(masked) = mask_sms_header(content) {
            output.push_str(&masked.line);
            output.push_str(ending);
            mask_sms_body = masked.following_body;
            continue;
        }

        let trimmed = content.trim();
        if mask_sms_body {
            if trimmed.is_empty() {
                output.push_str(content);
            } else if last_content_index == Some(line_index) && is_terminal_response_line(trimmed) {
                mask_sms_body = false;
                output.push_str(&mask_known_phone_line(content));
            } else {
                output.push_str("<masked sms body>");
            }
            output.push_str(ending);
            continue;
        }

        output.push_str(&mask_known_phone_line(content));
        output.push_str(ending);
    }

    output
}

fn split_line_ending(line: &str) -> (&str, &str) {
    if let Some(content) = line.strip_suffix("\r\n") {
        (content, "\r\n")
    } else if let Some(content) = line.strip_suffix('\n') {
        (content, "\n")
    } else {
        (line, "")
    }
}

struct MaskedSmsHeader {
    line: String,
    following_body: bool,
}

fn mask_sms_header(line: &str) -> Option<MaskedSmsHeader> {
    if let Some(fields) = csv_fields_after_prefix(line, "+CMGL:") {
        let status = fields.get(1).copied().unwrap_or_default();
        if numeric_sms_status_field(status) {
            return Some(MaskedSmsHeader {
                line: line.to_owned(),
                following_body: true,
            });
        }
        if text_sms_status_field(status) {
            if csv_field_is_quoted(&fields, 2) {
                return Some(MaskedSmsHeader {
                    line: mask_csv_field(line, fields, 2, mask_phone_number),
                    following_body: true,
                });
            }
            if csv_field_is_quoted(&fields, 4) {
                return Some(MaskedSmsHeader {
                    line: mask_csv_field(line, fields, 4, mask_phone_number),
                    following_body: false,
                });
            }
            if fields.len() == 4 {
                return Some(MaskedSmsHeader {
                    line: line.to_owned(),
                    following_body: true,
                });
            }
        }
    }
    if let Some(fields) = csv_fields_after_prefix(line, "+CMGR:") {
        let status = fields.first().copied().unwrap_or_default();
        if numeric_sms_status_field(status) {
            return Some(MaskedSmsHeader {
                line: line.to_owned(),
                following_body: true,
            });
        }
        if text_sms_status_field(status) {
            if csv_field_is_quoted(&fields, 1) {
                return Some(MaskedSmsHeader {
                    line: mask_csv_field(line, fields, 1, mask_phone_number),
                    following_body: true,
                });
            }
            if csv_field_is_quoted(&fields, 3) {
                return Some(MaskedSmsHeader {
                    line: mask_csv_field(line, fields, 3, mask_phone_number),
                    following_body: false,
                });
            }
            if fields.len() >= 8 {
                let masked = if csv_field_is_quoted(&fields, 5) {
                    mask_csv_field(line, fields, 5, mask_phone_number)
                } else {
                    line.to_owned()
                };
                return Some(MaskedSmsHeader {
                    line: masked,
                    following_body: true,
                });
            }
        }
    }
    if let Some(fields) = csv_fields_after_prefix(line, "+CMT:") {
        let masked = if fields.len() >= 3 && csv_field_is_quoted(&fields, 0) {
            mask_csv_field(line, fields, 0, mask_phone_number)
        } else {
            line.to_owned()
        };
        return Some(MaskedSmsHeader {
            line: masked,
            following_body: true,
        });
    }
    if let Some(fields) = csv_fields_after_prefix(line, "+CDS:") {
        if fields.len() == 1 {
            return Some(MaskedSmsHeader {
                line: line.to_owned(),
                following_body: true,
            });
        }
        let masked = if csv_field_is_quoted(&fields, 2) {
            mask_csv_field(line, fields, 2, mask_phone_number)
        } else {
            line.to_owned()
        };
        return Some(MaskedSmsHeader {
            line: masked,
            following_body: false,
        });
    }
    None
}

fn mask_known_phone_line(line: &str) -> String {
    const QUOTED_PHONE_RULES: &[(&str, usize)] = &[("+CLIP:", 0), ("+COLP:", 0), ("+CCWA:", 0)];
    const RULES: &[(&str, usize)] = &[
        ("AT+CMGS=", 0),
        ("AT+CMGW=", 0),
        ("AT+CMSS=", 1),
        ("AT+CSCA=", 0),
        ("AT+CPBW=", 1),
        ("AT+CCFC=", 2),
        ("+CNUM:", 1),
        ("+CSCA:", 0),
        ("+CPBR:", 1),
        ("+CPBF:", 1),
        ("+CCFC:", 2),
        ("+CLCC:", 5),
        ("+CSSU:", 2),
    ];

    for (prefix, index) in QUOTED_PHONE_RULES {
        if let Some(fields) = csv_fields_after_prefix(line, prefix)
            && fields.get(*index).is_some_and(|field| {
                let field = field.trim();
                field.len() >= 2 && field.starts_with('"') && field.ends_with('"')
            })
        {
            return mask_csv_field(line, fields, *index, mask_phone_number);
        }
    }
    for (prefix, index) in RULES {
        if let Some(fields) = csv_fields_after_prefix(line, prefix) {
            return mask_csv_field(line, fields, *index, mask_phone_number);
        }
    }
    if let Some(prefix_end) = find_ascii_case_insensitive_prefix(line, "ATD") {
        let value_start = skip_ascii_whitespace(line, prefix_end);
        if line[value_start..].chars().any(|ch| ch.is_ascii_digit()) {
            return format!(
                "{}{}",
                &line[..value_start],
                mask_phone_number(&line[value_start..])
            );
        }
    }
    line.to_owned()
}

fn mask_known_credential_fields(input: &str) -> String {
    const RULES: &[(&str, &[usize])] = &[
        ("AT+CPIN=", &[0, 1]),
        ("AT+CPIN2=", &[0, 1]),
        ("AT+CPWD=", &[1, 2]),
        ("AT+CLCK=", &[2]),
        ("AT+QICSGP=", &[3, 4]),
    ];

    input
        .split_inclusive('\n')
        .map(|line| {
            let (content, ending) = split_line_ending(line);
            let mut masked = content.to_owned();
            for (prefix, indexes) in RULES {
                if csv_fields_after_prefix(&masked, prefix).is_some() {
                    for index in *indexes {
                        let fields = csv_fields_after_prefix(&masked, prefix)
                            .expect("matched credential prefix remains stable while masking");
                        masked =
                            mask_csv_field(&masked, fields, *index, mask_declared_sensitive_value);
                    }
                    break;
                }
            }
            format!("{masked}{ending}")
        })
        .collect()
}

fn csv_fields_after_prefix<'a>(line: &'a str, prefix: &str) -> Option<Vec<&'a str>> {
    let prefix_end = find_ascii_case_insensitive_prefix(line, prefix)?;
    let params_start = skip_ascii_whitespace(line, prefix_end);
    Some(split_csv_fields(&line[params_start..]))
}

fn find_ascii_case_insensitive_prefix(value: &str, prefix: &str) -> Option<usize> {
    let leading = value.len() - value.trim_start().len();
    value[leading..]
        .get(..prefix.len())
        .filter(|candidate| candidate.eq_ignore_ascii_case(prefix))
        .map(|_| leading + prefix.len())
}

fn split_csv_fields(params: &str) -> Vec<&str> {
    let mut fields = Vec::new();
    let mut field_start = 0;
    let mut in_quotes = false;
    for (index, ch) in params.char_indices() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                fields.push(&params[field_start..index]);
                field_start = index + 1;
            }
            _ => {}
        }
    }
    fields.push(&params[field_start..]);
    fields
}

fn mask_csv_field(
    line: &str,
    fields: Vec<&str>,
    index: usize,
    masker: fn(&str) -> String,
) -> String {
    let Some(field) = fields.get(index) else {
        return line.to_owned();
    };
    let field_offset = field.as_ptr() as usize - line.as_ptr() as usize;
    let leading = field.len() - field.trim_start().len();
    let trailing = field.len() - field.trim_end().len();
    let core_start = field_offset + leading;
    let core_end = field_offset + field.len() - trailing;
    if core_start >= core_end {
        return line.to_owned();
    }

    let core = &line[core_start..core_end];
    let masked = if core.len() >= 2 && core.starts_with('"') && core.ends_with('"') {
        format!("\"{}\"", masker(&core[1..core.len() - 1]))
    } else {
        masker(core)
    };
    format!("{}{}{}", &line[..core_start], masked, &line[core_end..])
}

fn text_sms_status_field(field: &str) -> bool {
    let status = normalized_sms_status(field);
    matches!(
        status.as_str(),
        "REC UNREAD" | "REC READ" | "STO UNSENT" | "STO SENT" | "ALL"
    )
}

fn numeric_sms_status_field(field: &str) -> bool {
    matches!(
        normalized_sms_status(field).as_str(),
        "0" | "1" | "2" | "3" | "4"
    )
}

fn normalized_sms_status(field: &str) -> String {
    field.trim().trim_matches('"').to_ascii_uppercase()
}

fn csv_field_is_quoted(fields: &[&str], index: usize) -> bool {
    fields.get(index).is_some_and(|field| {
        let field = field.trim();
        field.len() >= 2 && field.starts_with('"') && field.ends_with('"')
    })
}

fn is_terminal_response_line(line: &str) -> bool {
    line == "OK"
        || line == "ERROR"
        || line == "NO CARRIER"
        || line.starts_with("+CME ERROR:")
        || line.starts_with("+CMS ERROR:")
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
    let fields = split_csv_fields(params);
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
    let leading_len = field.len() - field.trim_start().len();
    let trailing_len = field.len() - field.trim_end().len();
    let core_end = field.len() - trailing_len;
    let core = &field[leading_len..core_end];
    output.push_str(&field[..leading_len]);
    if core.len() >= 2 && core.starts_with('"') && core.ends_with('"') {
        output.push('"');
        output.push_str(&mask_declared_sensitive_value(&core[1..core.len() - 1]));
        output.push('"');
    } else {
        output.push_str(&mask_declared_sensitive_value(core));
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
    let value_start = skip_ascii_whitespace(line, prefix_end);
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
    fn fully_masks_cgauth_credentials() {
        assert_eq!(
            mask_sensitive_values("AT+CGAUTH=1,2,\"sora\",\"custom-password\""),
            "AT+CGAUTH=1,2,\"****\",\"******-********\""
        );
    }

    #[test]
    fn leaves_cgauth_capability_fields_plain() {
        assert_eq!(mask_sensitive_values("AT+CGAUTH=?"), "AT+CGAUTH=?");
        assert_eq!(
            mask_sensitive_values("+CGAUTH: (1-16),(0-3),(0-64),(0-64)"),
            "+CGAUTH: (1-16),(0-3),(0-64),(0-64)"
        );
    }

    #[test]
    fn masks_short_phone_fields_and_preserves_operational_neighbors() {
        let input = concat!(
            "+CNUM: \"line\",\"901001\",129\n",
            "+CMGL: 3,\"REC UNREAD\",\"+1-23*45#\",,\"26/07/18,12:00:00+36\"\n",
            "secret body 42\n",
            "OK"
        );
        let masked = mask_sensitive_values(input);
        assert!(masked.contains("+CNUM: \"line\",\"******\",129"));
        assert!(masked.contains("+CMGL: 3,\"REC UNREAD\",\"+*-*****#\""));
        assert!(masked.contains("<masked sms body>\nOK"));
        assert_eq!(mask_sensitive_values("+CMGS: 7"), "+CMGS: 7");
    }

    #[test]
    fn fully_masks_text_and_pdu_sms_bodies_that_look_like_protocol_lines() {
        let text = concat!(
            "+CMGL: 3,\"REC READ\",\"901001\"\n",
            "AT+SECRET\n",
            "+CMGL: 4,\"REC READ\",\"901002\"\n",
            "OK\n",
            "OK"
        );
        assert_eq!(
            mask_sensitive_values(text),
            concat!(
                "+CMGL: 3,\"REC READ\",\"******\"\n",
                "<masked sms body>\n",
                "+CMGL: 4,\"REC READ\",\"******\"\n",
                "<masked sms body>\n",
                "OK"
            )
        );

        let pdu = "+CMGR: 0,,23\n00110006910001F10000AA05C8329BFD06\nOK";
        assert_eq!(
            mask_sensitive_values(pdu),
            "+CMGR: 0,,23\n<masked sms body>\nOK"
        );
    }

    #[test]
    fn distinguishes_sms_addresses_bodies_and_operational_header_fields() {
        let status_report = concat!(
            "+CMGL: 3,\"REC READ\",2,7,\"901003\",129,\"26/07/18,12:00:00+36\",\"26/07/18,12:00:01+36\",0\n",
            "OK"
        );
        assert_eq!(
            mask_sensitive_values(status_report),
            concat!(
                "+CMGL: 3,\"REC READ\",2,7,\"******\",129,\"26/07/18,12:00:00+36\",\"26/07/18,12:00:01+36\",0\n",
                "OK"
            )
        );

        let cell_broadcast = "+CMGL: 1,\"REC READ\",100,40,1,3\nWeather 20 C\nOK";
        assert_eq!(mask_sensitive_values(cell_broadcast), cell_broadcast);

        let incoming = "+CMT: \"901004\",,\"26/07/18,12:00:00+36\"\nOK\nOK";
        assert_eq!(
            mask_sensitive_values(incoming),
            "+CMT: \"******\",,\"26/07/18,12:00:00+36\"\n<masked sms body>\nOK"
        );

        let delivery =
            "+CDS: 2,7,\"901005\",129,\"26/07/18,12:00:00+36\",\"26/07/18,12:00:01+36\",0";
        assert_eq!(
            mask_sensitive_values(delivery),
            "+CDS: 2,7,\"******\",129,\"26/07/18,12:00:00+36\",\"26/07/18,12:00:01+36\",0"
        );
    }

    #[test]
    fn distinguishes_cmgs_destination_from_message_reference() {
        assert_eq!(mask_sensitive_values("AT+CMGS=901001"), "AT+CMGS=******");
        assert_eq!(mask_sensitive_values("+CMGS: 12"), "+CMGS: 12");
    }

    #[test]
    fn fully_masks_short_known_credentials_and_dial_destinations() {
        assert_eq!(mask_sensitive_values("AT+CPIN=1234"), "AT+CPIN=****");
        assert_eq!(
            mask_sensitive_values("AT+CPWD=\"SC\",\"1234\",\"5678\""),
            "AT+CPWD=\"SC\",\"****\",\"****\""
        );
        assert_eq!(
            mask_sensitive_values("AT+CLCK=\"SC\",1,\"1234\",7"),
            "AT+CLCK=\"SC\",1,\"****\",7"
        );
        assert_eq!(mask_sensitive_values("ATD*901001#;"), "ATD*******#;");
    }

    #[test]
    fn leaves_short_operational_and_unknown_vendor_values_plain() {
        assert_eq!(mask_sensitive_values("+CSQ: 18,99"), "+CSQ: 18,99");
        assert_eq!(mask_sensitive_values("+CLIP: 1,1"), "+CLIP: 1,1");
        assert_eq!(mask_sensitive_values("+CCWA: 1,1"), "+CCWA: 1,1");
        assert_eq!(mask_sensitive_values("+VENDOR: 901001"), "+VENDOR: 901001");
    }

    #[test]
    fn masks_known_call_number_fields_without_hiding_status_fields() {
        assert_eq!(
            mask_sensitive_values("+CLIP: \"901001\",129,,,,0"),
            "+CLIP: \"******\",129,,,,0"
        );
        assert_eq!(
            mask_sensitive_values("+CCFC: 1,1,\"901002\",129"),
            "+CCFC: 1,1,\"******\",129"
        );
    }

    #[test]
    fn masks_declared_values_without_retaining_content() {
        assert_eq!(
            mask_declared_sensitive_value("token-42/example"),
            "*****-**/*******"
        );
        assert_eq!(mask_phone_number("+1 (23)*45#"), "+* (**)***#");
    }
}
