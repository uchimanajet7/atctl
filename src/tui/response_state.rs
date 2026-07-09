#[derive(Debug, Clone)]
pub(super) struct ResponseState {
    pub(super) masked_text: String,
    pub(super) raw_text: Option<String>,
}

impl ResponseState {
    pub(super) fn masked(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            masked_text: sanitize_response_text_for_tui(&text),
            raw_text: None,
        }
    }

    pub(super) fn with_raw(masked_text: impl Into<String>, raw_text: impl Into<String>) -> Self {
        let masked_text = masked_text.into();
        let masked_text = sanitize_response_text_for_tui(&masked_text);
        let raw_text = raw_text.into();
        let raw_text = sanitize_response_text_for_tui(&raw_text);
        let raw_text = (raw_text != masked_text).then_some(raw_text);
        Self {
            masked_text,
            raw_text,
        }
    }

    pub(super) fn visible_text(&self, output_masking_enabled: bool) -> &str {
        if output_masking_enabled {
            &self.masked_text
        } else {
            self.raw_text.as_deref().unwrap_or(&self.masked_text)
        }
    }

    #[cfg(test)]
    pub(super) fn contains(&self, needle: &str) -> bool {
        self.visible_text(true).contains(needle)
    }

    #[cfg(test)]
    pub(super) fn contains_visible(&self, output_masking_enabled: bool, needle: &str) -> bool {
        self.visible_text(output_masking_enabled).contains(needle)
    }

    pub(super) fn has_raw_text(&self) -> bool {
        self.raw_text.is_some()
    }

    #[cfg(test)]
    pub(super) fn output_masking_label(
        &self,
        output_masking_enabled: bool,
    ) -> Option<&'static str> {
        if self.raw_text.is_none() {
            None
        } else if output_masking_enabled {
            Some("on")
        } else {
            Some("off")
        }
    }

    pub(super) fn clear_raw(&mut self) {
        self.raw_text = None;
    }

    pub(super) fn clear(&mut self) {
        self.masked_text.clear();
        self.clear_raw();
    }

    pub(super) fn is_empty(&self) -> bool {
        self.masked_text.is_empty()
    }
}

fn sanitize_response_text_for_tui(text: &str) -> String {
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    enum EscapeState {
        None,
        Escape,
        Csi,
        Osc,
        OscEscape,
    }

    let mut output = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut escape_state = EscapeState::None;

    while let Some(ch) = chars.next() {
        match escape_state {
            EscapeState::None => match ch {
                '\r' => {
                    if chars.peek() != Some(&'\n') {
                        output.push('\n');
                    }
                }
                '\n' => output.push('\n'),
                '\t' => output.push_str("    "),
                '\u{1b}' => escape_state = EscapeState::Escape,
                value if value.is_control() => {}
                value => output.push(value),
            },
            EscapeState::Escape => {
                escape_state = match ch {
                    '[' => EscapeState::Csi,
                    ']' => EscapeState::Osc,
                    _ => EscapeState::None,
                };
            }
            EscapeState::Csi => {
                if ('@'..='~').contains(&ch) {
                    escape_state = EscapeState::None;
                }
            }
            EscapeState::Osc => {
                escape_state = match ch {
                    '\u{7}' => EscapeState::None,
                    '\u{1b}' => EscapeState::OscEscape,
                    _ => EscapeState::Osc,
                };
            }
            EscapeState::OscEscape => {
                escape_state = if ch == '\\' {
                    EscapeState::None
                } else {
                    EscapeState::Osc
                };
            }
        }
    }

    output
}
