use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph, Sparkline},
    Frame,
};

use crate::ui::theme;

pub(crate) fn resource_card(
    frame: &mut Frame,
    area: Rect,
    card: &(&str, String, String, Option<f64>, Vec<u64>),
    focused: bool,
    palette: &theme::Palette,
) {
    let (title, main, secondary, usage, history) = card;
    if area.width < 3 || area.height < 2 {
        return;
    }
    let border = if focused {
        palette.accent
    } else {
        palette.border
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .title_style(
            Style::default()
                .fg(palette.text_muted)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().bg(palette.surface).fg(palette.text))
        .border_style(Style::default().fg(border));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let color = usage
        .map(|value| usage_color(value, palette))
        .unwrap_or(palette.accent);
    frame.render_widget(
        Paragraph::new(main.to_owned()).style(
            Style::default()
                .fg(color)
                .bg(palette.surface)
                .add_modifier(Modifier::BOLD),
        ),
        Rect { height: 1, ..inner },
    );
    if inner.height > 1 {
        frame.render_widget(
            Paragraph::new(secondary.to_owned())
                .style(Style::default().fg(palette.text_muted).bg(palette.surface)),
            Rect {
                y: inner.y + 1,
                height: 1,
                ..inner
            },
        );
    }
    if inner.height > 2 && !history.is_empty() {
        let spark_area = Rect {
            y: inner.y + 2,
            height: inner.height - 2,
            ..inner
        };
        frame.render_widget(
            Sparkline::default()
                .data(history)
                .max(if usage.is_some() {
                    100
                } else {
                    history.iter().copied().max().unwrap_or(1).max(1)
                })
                .style(Style::default().fg(color).bg(palette.surface)),
            spark_area,
        );
    }
}

pub(crate) fn network_card(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    main: String,
    color: ratatui::style::Color,
    history: &[u64],
    palette: &theme::Palette,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .title_style(
            Style::default()
                .fg(palette.text_muted)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(palette.border))
        .style(Style::default().bg(palette.surface));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(main).style(
            Style::default()
                .fg(color)
                .bg(palette.surface)
                .add_modifier(Modifier::BOLD),
        ),
        inner,
    );
    if inner.height > 1 && !history.is_empty() {
        frame.render_widget(
            Sparkline::default()
                .data(history)
                .max(history.iter().copied().max().unwrap_or(1).max(1))
                .style(Style::default().fg(color).bg(palette.surface)),
            Rect {
                y: inner.y + 1,
                height: inner.height - 1,
                ..inner
            },
        );
    }
}

pub(crate) fn usage_color(value: f64, palette: &theme::Palette) -> ratatui::style::Color {
    if value >= 85.0 {
        palette.danger
    } else if value >= 70.0 {
        palette.warn
    } else {
        palette.ok
    }
}
