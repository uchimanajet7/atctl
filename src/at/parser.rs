use crate::at::response::{AtResponse, AtStatus};

pub fn parse_response(raw: &[u8]) -> AtResponse {
    let text = String::from_utf8_lossy(raw).into_owned();
    let lines = split_lines(&text);
    let status = detect_status(&lines);

    AtResponse {
        raw: raw.to_vec(),
        text,
        lines,
        status,
    }
}

fn split_lines(text: &str) -> Vec<String> {
    text.split(['\r', '\n'])
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn detect_status(lines: &[String]) -> AtStatus {
    for line in lines.iter().rev() {
        if line == "OK" {
            return AtStatus::Ok;
        }
        if line == "ERROR" {
            return AtStatus::Error;
        }
        if line.starts_with("+CME ERROR:") {
            return AtStatus::CmeError(line.clone());
        }
        if line.starts_with("+CMS ERROR:") {
            return AtStatus::CmsError(line.clone());
        }
        if line == "NO CARRIER" {
            return AtStatus::NoCarrier;
        }
    }

    AtStatus::Incomplete
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_ok_with_echo() {
        let response = parse_response(b"AT\r\r\nOK\r\n");

        assert_eq!(response.lines, ["AT", "OK"]);
        assert_eq!(response.status, AtStatus::Ok);
    }

    #[test]
    fn detects_cme_error() {
        let response = parse_response(b"\r\n+CME ERROR: 10\r\n");

        assert_eq!(response.status, AtStatus::CmeError("+CME ERROR: 10".into()));
    }

    #[test]
    fn preserves_incomplete_payload() {
        let response = parse_response(b"\r\n+QENG: servingcell\r\n");

        assert_eq!(response.status, AtStatus::Incomplete);
        assert!(response.text.contains("+QENG"));
    }
}
