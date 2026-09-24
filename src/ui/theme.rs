use ratatui::style::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Forest,
    Amber,
    Mono,
    Solarized,
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub background: Color,
    pub panel: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub olive: Color,
    pub green: Color,
    pub yellow: Color,
    pub red: Color,
}

impl Theme {
    pub const ALL: [Self; 4] = [Self::Forest, Self::Amber, Self::Mono, Self::Solarized];

    pub fn palette(self) -> Palette {
        match self {
            Self::Forest => Palette {
                background: Color::Rgb(9, 13, 10),
                panel: Color::Rgb(19, 26, 20),
                border: Color::Rgb(66, 78, 56),
                text: Color::Rgb(232, 235, 216),
                muted: Color::Rgb(148, 158, 135),
                olive: Color::Rgb(164, 191, 101),
                green: Color::Rgb(137, 207, 138),
                yellow: Color::Rgb(235, 194, 91),
                red: Color::Rgb(232, 117, 100),
            },
            Self::Amber => Palette {
                background: Color::Rgb(20, 16, 10),
                panel: Color::Rgb(31, 25, 16),
                border: Color::Rgb(92, 68, 35),
                text: Color::Rgb(245, 232, 202),
                muted: Color::Rgb(164, 145, 112),
                olive: Color::Rgb(235, 181, 74),
                green: Color::Rgb(183, 205, 119),
                yellow: Color::Rgb(245, 194, 78),
                red: Color::Rgb(232, 117, 84),
            },
            Self::Mono => Palette {
                background: Color::Rgb(12, 12, 12),
                panel: Color::Rgb(25, 25, 25),
                border: Color::Rgb(82, 82, 82),
                text: Color::Rgb(235, 235, 235),
                muted: Color::Rgb(158, 158, 158),
                olive: Color::Rgb(210, 210, 210),
                green: Color::Rgb(205, 205, 205),
                yellow: Color::Rgb(235, 235, 235),
                red: Color::Rgb(255, 255, 255),
            },
            Self::Solarized => Palette {
                background: Color::Rgb(0, 43, 54),
                panel: Color::Rgb(7, 54, 66),
                border: Color::Rgb(42, 94, 104),
                text: Color::Rgb(238, 232, 213),
                muted: Color::Rgb(147, 161, 161),
                olive: Color::Rgb(181, 137, 0),
                green: Color::Rgb(133, 153, 0),
                yellow: Color::Rgb(203, 75, 22),
                red: Color::Rgb(220, 50, 47),
            },
        }
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

pub const BACKGROUND: Color = Color::Rgb(9, 13, 10);
pub const PANEL: Color = Color::Rgb(19, 26, 20);
pub const BORDER: Color = Color::Rgb(66, 78, 56);
pub const TEXT: Color = Color::Rgb(232, 235, 216);
pub const MUTED: Color = Color::Rgb(148, 158, 135);
pub const OLIVE: Color = Color::Rgb(164, 191, 101);
pub const GREEN: Color = Color::Rgb(137, 207, 138);
pub const YELLOW: Color = Color::Rgb(235, 194, 91);
pub const RED: Color = Color::Rgb(232, 117, 100);
