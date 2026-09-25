use filiz::ui::theme::Theme;
use ratatui::style::{Color, Modifier};

#[test]
fn forest_keeps_existing_colors() {
    let p = Theme::Forest.palette();
    assert_eq!(p.bg, Color::Rgb(9, 13, 10));
    assert_eq!(p.surface, Color::Rgb(19, 26, 20));
    assert_eq!(p.border, Color::Rgb(66, 78, 56));
    assert_eq!(p.text, Color::Rgb(232, 235, 216));
    assert_eq!(p.text_muted, Color::Rgb(148, 158, 135));
    assert_eq!(p.accent, Color::Rgb(164, 191, 101));
    assert_eq!(p.ok, Color::Rgb(137, 207, 138));
    assert_eq!(p.warn, Color::Rgb(235, 194, 91));
    assert_eq!(p.danger, Color::Rgb(232, 117, 100));
}

#[test]
fn every_theme_distinguishes_danger_from_text() {
    for theme in Theme::ALL {
        let p = theme.palette();
        assert!(
            p.danger != p.text || p.danger_modifier.contains(Modifier::REVERSED),
            "{theme:?} danger is indistinguishable"
        );
    }
}

#[test]
fn mono_marks_danger_with_reverse_video() {
    assert!(Theme::Mono
        .palette()
        .danger_modifier
        .contains(Modifier::REVERSED | Modifier::BOLD));
}

#[test]
fn theme_cycle_visits_all_four_themes() {
    let mut theme = Theme::Forest;
    let mut seen = vec![theme];
    for _ in 0..3 {
        theme = theme.next();
        seen.push(theme);
    }
    assert_eq!(seen, Theme::ALL.to_vec());
    assert_eq!(theme.next(), Theme::Forest);
}
