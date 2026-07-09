use ratatui::prelude::{Color, Modifier, Style};

use crate::at::risk::RiskLevel;
use crate::cli::TuiThemeChoice;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) enum TuiStyleRole {
    Background,
    Text,
    Focus,
    Selected,
    Status,
    Muted,
    RiskSafe,
    RiskSensitive,
    RiskWrite,
    RiskPersistent,
    RiskDangerous,
    RiskUnknown,
    Warning,
    #[allow(dead_code)]
    Error,
}

#[cfg(test)]
pub(super) const REQUIRED_STYLE_ROLES: [TuiStyleRole; 14] = [
    TuiStyleRole::Background,
    TuiStyleRole::Text,
    TuiStyleRole::Focus,
    TuiStyleRole::Selected,
    TuiStyleRole::Status,
    TuiStyleRole::Muted,
    TuiStyleRole::RiskSafe,
    TuiStyleRole::RiskSensitive,
    TuiStyleRole::RiskWrite,
    TuiStyleRole::RiskPersistent,
    TuiStyleRole::RiskDangerous,
    TuiStyleRole::RiskUnknown,
    TuiStyleRole::Warning,
    TuiStyleRole::Error,
];

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) struct TuiTheme {
    mode: TuiThemeMode,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) enum TuiThemeMode {
    Dark,
    Light,
    NoColor,
}

impl TuiTheme {
    pub(super) fn from_choice(choice: Option<TuiThemeChoice>) -> Self {
        let no_color = std::env::var_os("NO_COLOR");
        let no_color = no_color.as_ref().map(|value| value.to_string_lossy());
        Self::from_choice_and_no_color_value(choice, no_color.as_deref())
    }

    pub(super) const fn dark() -> Self {
        Self {
            mode: TuiThemeMode::Dark,
        }
    }

    pub(super) const fn light() -> Self {
        Self {
            mode: TuiThemeMode::Light,
        }
    }

    pub(super) const fn no_color() -> Self {
        Self {
            mode: TuiThemeMode::NoColor,
        }
    }

    #[cfg(test)]
    pub(super) const fn colored() -> Self {
        Self::dark()
    }

    #[cfg(test)]
    pub(super) const fn mode(self) -> TuiThemeMode {
        self.mode
    }

    pub(super) fn from_choice_and_no_color_value(
        choice: Option<TuiThemeChoice>,
        no_color_value: Option<&str>,
    ) -> Self {
        match choice {
            Some(TuiThemeChoice::Dark) => Self::dark(),
            Some(TuiThemeChoice::Light) => Self::light(),
            Some(TuiThemeChoice::NoColor) => Self::no_color(),
            None if no_color_value.is_some() => Self::no_color(),
            None => Self::dark(),
        }
    }

    pub(super) fn risk_style(self, risk: RiskLevel) -> Style {
        match risk {
            RiskLevel::Safe => self.style(TuiStyleRole::RiskSafe),
            RiskLevel::Sensitive => self.style(TuiStyleRole::RiskSensitive),
            RiskLevel::Write => self.style(TuiStyleRole::RiskWrite),
            RiskLevel::Persistent => self.style(TuiStyleRole::RiskPersistent),
            RiskLevel::Dangerous => self.style(TuiStyleRole::RiskDangerous),
            RiskLevel::Unknown => self.style(TuiStyleRole::RiskUnknown),
        }
    }

    pub(super) fn style(self, role: TuiStyleRole) -> Style {
        match self.mode {
            TuiThemeMode::Dark => palette_style(dark_palette(), role),
            TuiThemeMode::Light => palette_style(light_palette(), role),
            TuiThemeMode::NoColor => no_color_style(role),
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct Palette {
    background: Color,
    text: Color,
    focus: Color,
    selected: Color,
    safe: Color,
    sensitive: Color,
    write: Color,
    persistent: Color,
    dangerous: Color,
    unknown: Color,
}

const fn rgb(red: u8, green: u8, blue: u8) -> Color {
    Color::Rgb(red, green, blue)
}

fn dark_palette() -> Palette {
    Palette {
        background: rgb(0x26, 0x32, 0x38),
        text: rgb(0xec, 0xef, 0xf1),
        focus: rgb(0x4d, 0xd0, 0xe1),
        selected: rgb(0xff, 0xd5, 0x4f),
        safe: rgb(0x4d, 0xd0, 0xe1),
        sensitive: rgb(0xd6, 0xb3, 0xff),
        write: rgb(0xff, 0xd1, 0x66),
        persistent: rgb(0xff, 0xb8, 0x6c),
        dangerous: rgb(0xff, 0x6b, 0x6b),
        unknown: rgb(0xb0, 0xbe, 0xc5),
    }
}

fn light_palette() -> Palette {
    Palette {
        background: rgb(0xfa, 0xfa, 0xfa),
        text: rgb(0x26, 0x32, 0x38),
        focus: rgb(0x00, 0x7c, 0x89),
        selected: rgb(0x7a, 0x5a, 0x00),
        safe: rgb(0x00, 0x7c, 0x89),
        sensitive: rgb(0x6b, 0x3f, 0xa0),
        write: rgb(0x7a, 0x5a, 0x00),
        persistent: rgb(0x9a, 0x4d, 0x00),
        dangerous: rgb(0xb0, 0x00, 0x20),
        unknown: rgb(0x4b, 0x55, 0x63),
    }
}

fn palette_style(palette: Palette, role: TuiStyleRole) -> Style {
    let base = Style::default().bg(palette.background);
    match role {
        TuiStyleRole::Background => base,
        TuiStyleRole::Text => base.fg(palette.text),
        TuiStyleRole::Focus => base.fg(palette.focus).add_modifier(Modifier::BOLD),
        TuiStyleRole::Selected => base.fg(palette.selected).add_modifier(Modifier::BOLD),
        TuiStyleRole::Status => base.fg(palette.focus),
        TuiStyleRole::Muted => base.fg(palette.unknown),
        TuiStyleRole::RiskSafe => base.fg(palette.safe),
        TuiStyleRole::RiskSensitive => base.fg(palette.sensitive),
        TuiStyleRole::RiskWrite => base.fg(palette.write),
        TuiStyleRole::RiskPersistent => base.fg(palette.persistent),
        TuiStyleRole::RiskDangerous => base.fg(palette.dangerous),
        TuiStyleRole::RiskUnknown => base.fg(palette.unknown),
        TuiStyleRole::Warning => base.fg(palette.write),
        TuiStyleRole::Error => base.fg(palette.dangerous),
    }
}

fn no_color_style(role: TuiStyleRole) -> Style {
    match role {
        TuiStyleRole::Background | TuiStyleRole::Text => Style::default(),
        TuiStyleRole::Focus | TuiStyleRole::Selected => {
            Style::default().add_modifier(Modifier::BOLD)
        }
        TuiStyleRole::Status
        | TuiStyleRole::Muted
        | TuiStyleRole::RiskSafe
        | TuiStyleRole::RiskSensitive
        | TuiStyleRole::RiskWrite
        | TuiStyleRole::RiskPersistent
        | TuiStyleRole::RiskDangerous
        | TuiStyleRole::RiskUnknown
        | TuiStyleRole::Warning
        | TuiStyleRole::Error => Style::default(),
    }
}
