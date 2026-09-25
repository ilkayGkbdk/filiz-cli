use ratatui::style::{Color, Modifier};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Forest,
    Amber,
    Mono,
    Solarized,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub bg: Color,
    pub surface: Color,
    pub surface_alt: Color,
    pub border: Color,
    pub border_focus: Color,
    pub text: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub accent_fg: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub ok: Color,
    pub warn: Color,
    pub danger: Color,
    pub info: Color,
    pub chart_rx: Color,
    pub chart_tx: Color,
    pub danger_modifier: Modifier,
}

struct Base {
    bg: (u8, u8, u8),
    surface: (u8, u8, u8),
    border: (u8, u8, u8),
    text: (u8, u8, u8),
    muted: (u8, u8, u8),
    accent: (u8, u8, u8),
    ok: (u8, u8, u8),
    warn: (u8, u8, u8),
    danger: (u8, u8, u8),
    danger_modifier: Modifier,
}

fn rgb((r, g, b): (u8, u8, u8)) -> Color {
    Color::Rgb(r, g, b)
}

impl Base {
    fn palette(self) -> Palette {
        Palette {
            bg: rgb(self.bg),
            surface: rgb(self.surface),
            surface_alt: rgb(self.surface),
            border: rgb(self.border),
            border_focus: rgb(self.accent),
            text: rgb(self.text),
            text_muted: rgb(self.muted),
            accent: rgb(self.accent),
            accent_fg: rgb(self.bg),
            selection_bg: rgb(self.accent),
            selection_fg: rgb(self.bg),
            ok: rgb(self.ok),
            warn: rgb(self.warn),
            danger: rgb(self.danger),
            info: rgb(self.ok),
            chart_rx: rgb(self.ok),
            chart_tx: rgb(self.accent),
            danger_modifier: self.danger_modifier,
        }
    }
}

impl Theme {
    pub const ALL: [Self; 4] = [Self::Forest, Self::Amber, Self::Mono, Self::Solarized];

    pub fn palette(self) -> Palette {
        match self {
            Self::Forest => Base {
                bg: (9, 13, 10),
                surface: (19, 26, 20),
                border: (66, 78, 56),
                text: (232, 235, 216),
                muted: (148, 158, 135),
                accent: (164, 191, 101),
                ok: (137, 207, 138),
                warn: (235, 194, 91),
                danger: (232, 117, 100),
                danger_modifier: Modifier::BOLD,
            },
            Self::Amber => Base {
                bg: (20, 16, 10),
                surface: (31, 25, 16),
                border: (92, 68, 35),
                text: (245, 232, 202),
                muted: (164, 145, 112),
                accent: (235, 181, 74),
                ok: (183, 205, 119),
                warn: (245, 194, 78),
                danger: (232, 117, 84),
                danger_modifier: Modifier::BOLD,
            },
            Self::Mono => Base {
                bg: (12, 12, 12),
                surface: (25, 25, 25),
                border: (82, 82, 82),
                text: (235, 235, 235),
                muted: (158, 158, 158),
                accent: (210, 210, 210),
                ok: (205, 205, 205),
                warn: (235, 235, 235),
                danger: (255, 255, 255),
                danger_modifier: Modifier::BOLD.union(Modifier::REVERSED),
            },
            Self::Solarized => Base {
                bg: (0, 43, 54),
                surface: (7, 54, 66),
                border: (42, 94, 104),
                text: (238, 232, 213),
                muted: (147, 161, 161),
                accent: (181, 137, 0),
                ok: (133, 153, 0),
                warn: (203, 75, 22),
                danger: (220, 50, 47),
                danger_modifier: Modifier::BOLD,
            },
        }
        .palette()
    }

    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|theme| *theme == self)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Forest => "Forest",
            Self::Amber => "Amber",
            Self::Mono => "Mono",
            Self::Solarized => "Solarized",
        }
    }
}
