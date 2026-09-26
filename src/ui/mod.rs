//! Rendering. Every frame is drawn from `App` state; nothing here mutates
//! anything except table scroll offsets.

mod dashboard;
pub mod fmt;
mod host;
mod hosts;
mod problems;
mod widgets;

use crate::app::{App, HostTab, Modal, ToastKind, View};
use chrono::Local;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph, Wrap};
use widgets::{centered, truncate};

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let [header, body, footer] = Layout::vertical([Constraint::Length(2), Constraint::Min(0), Constraint::Length(1)]).areas(area);

    draw_header(f, header, app);
    if app.snapshot.is_none() {
        draw_splash(f, body, app);
    } else {
        match app.view {
            View::Dashboard => dashboard::draw(f, body, app),
            View::Hosts => hosts::draw(f, body, app),
            View::Problems => problems::draw(f, body, app),
            View::Host => host::draw(f, body, app),
        }
    }
    draw_footer(f, footer, app);
    draw_toasts(f, body, app);
    match &app.modal {
        Some(Modal::Help) => draw_help(f, area, app),
        Some(Modal::Ack { .. }) => draw_ack(f, area, app),
        None => {}
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let [line, _] = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);

    let mut left = vec![Span::styled(" ◆ zabterm ", Style::new().fg(t.on_badge()).bg(t.accent).bold()), Span::raw("  ")];
    let (hosts, problems, worst) = match &app.snapshot {
        Some(s) => {
            let active: Vec<_> = s.problems.iter().filter(|p| !p.suppressed).collect();
            (s.hosts.len(), active.len(), active.iter().map(|p| p.severity).max())
        }
        None => (0, 0, None),
    };
    let tabs = [(View::Dashboard, "1", "Dashboard", None), (View::Hosts, "2", "Hosts", Some(hosts)), (View::Problems, "3", "Problems", Some(problems))];
    for (view, key, label, count) in tabs {
        // While on a host page, keep the list the user came from lit.
        let active = app.view == view || (app.view == View::Host && view == app.back_to);
        let style = if active { Style::new().fg(t.accent).bold().add_modifier(Modifier::UNDERLINED) } else { Style::new().fg(t.muted) };
        left.push(Span::styled(format!("{key} "), Style::new().fg(t.border)));
        left.push(Span::styled(label, style));
        if let Some(n) = count {
            let color = if view == View::Problems { worst.map(|s| t.sev[s as usize]).unwrap_or(t.ok) } else { t.muted };
            left.push(Span::styled(format!(" {n}"), Style::new().fg(color).bold()));
        }
        left.push(Span::raw("   "));
    }
    if app.view == View::Host {
        let name = app.detail_row().map(|h| h.name.clone()).unwrap_or_default();
        left.push(Span::styled("› ", Style::new().fg(t.border)));
        left.push(Span::styled(name, Style::new().fg(t.accent).bold().add_modifier(Modifier::UNDERLINED)));
    }

    let (dot, state) = if app.error.is_some() {
        (Span::styled("●", Style::new().fg(t.err)), Span::styled(" offline", Style::new().fg(t.err)))
    } else if app.loading {
        (Span::styled(SPINNER[(app.frame % 10) as usize], Style::new().fg(t.accent)), Span::styled(" sync", Style::new().fg(t.muted)))
    } else {
        let pulse = if app.frame % 8 < 4 { t.ok } else { t.muted };
        (Span::styled("●", Style::new().fg(pulse)), Span::styled(" live", Style::new().fg(t.muted)))
    };
    let sep = || Span::styled("  ·  ", Style::new().fg(t.border));
    let clock = Span::styled(Local::now().format("%H:%M:%S").to_string(), Style::new().fg(t.fg));
    let profile = Span::styled(app.profile.name.clone(), Style::new().fg(t.fg).bold());
    let version = app.version.as_ref().map(|v| Span::styled(format!("Zabbix {v}"), Style::new().fg(t.muted)));
    let latency = app.snapshot.as_ref().map(|s| Span::styled(format!("{}ms", s.latency_ms), Style::new().fg(t.muted)));

    // Drop the least important bits first when the terminal is narrow.
    let left = Line::from(left);
    let tiers: [Vec<Option<Span>>; 3] =
        [vec![Some(profile.clone()), version, latency, Some(clock.clone())], vec![Some(profile), Some(clock.clone())], vec![Some(clock)]];
    let mut right = Line::default();
    for tier in tiers {
        let mut spans = vec![dot.clone(), state.clone()];
        for part in tier.into_iter().flatten() {
            spans.push(sep());
            spans.push(part);
        }
        spans.push(Span::raw(" "));
        right = Line::from(spans);
        if left.width() + right.width() < area.width as usize {
            break;
        }
    }
    f.render_widget(Paragraph::new(left), line);
    f.render_widget(Paragraph::new(right).alignment(Alignment::Right), line);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    if app.editing_filter {
        let text = match app.view {
            View::Hosts => &app.host_filter,
            View::Problems => &app.problem_filter,
            _ => &app.item_filter,
        };
        let line = Line::from(vec![
            Span::styled(" / ", Style::new().fg(t.on_badge()).bg(t.accent).bold()),
            Span::raw(" "),
            Span::styled(text.clone(), Style::new().fg(t.fg)),
            Span::styled("▏", Style::new().fg(t.accent).add_modifier(Modifier::SLOW_BLINK)),
            Span::styled("   ⏎ keep  esc clear", Style::new().fg(t.muted)),
        ]);
        f.render_widget(Paragraph::new(line), area);
        return;
    }
    let hints: &[(&str, &str)] = match app.view {
        View::Dashboard => &[("j/k", "select"), ("⏎", "open"), ("m", "cpu/mem"), ("tab", "next"), ("t", "theme"), ("o", "web"), ("?", "help"), ("q", "quit")],
        View::Hosts => &[("j/k", "move"), ("⏎", "open"), ("/", "filter"), ("s", "sort"), ("o", "web"), ("r", "refresh"), ("?", "help")],
        View::Problems => &[("j/k", "move"), ("a", "ack"), ("⏎", "host"), ("+/-", "severity"), ("h", "hide acked"), ("/", "filter"), ("?", "help")],
        View::Host if app.host_tab == HostTab::Items => &[("esc", "back"), ("tab", "graphs"), ("/", "filter"), ("j/k", "move"), ("o", "web"), ("?", "help")],
        View::Host => &[("esc", "back"), ("[ ]", "range"), ("tab", "items"), ("r", "refresh"), ("o", "web"), ("?", "help")],
    };
    let mut spans = vec![Span::raw(" ")];
    for (key, label) in hints {
        spans.push(Span::styled(*key, Style::new().fg(t.accent).bold()));
        spans.push(Span::styled(format!(" {label}   "), Style::new().fg(t.muted)));
    }
    let hints = Line::from(spans);
    f.render_widget(Paragraph::new(hints.clone()), area);

    if let Some(s) = &app.snapshot {
        let ago = (Local::now() - s.fetched_at).num_seconds();
        let right = Line::from(vec![
            Span::styled(format!("updated {}s ago", ago.max(0)), Style::new().fg(t.muted)),
            Span::styled(format!("  every {}s ", app.config.refresh_interval), Style::new().fg(t.border)),
        ]);
        if hints.width() + right.width() < area.width as usize {
            f.render_widget(Paragraph::new(right).alignment(Alignment::Right), area);
        }
    }
}

/// "ZABTERM" in the same half-block style as the dashboard digits.
const LOGO: [[&str; 3]; 7] = [
    ["▀▀█", "▄▀ ", "▀▀▀"],
    ["▄▀▄", "█▀█", "▀ ▀"],
    ["█▀▄", "█▀▄", "▀▀ "],
    ["▀█▀", " █ ", " ▀ "],
    ["█▀▀", "█▀▀", "▀▀▀"],
    ["█▀▄", "█▀▄", "▀ ▀"],
    ["█▄ ▄█", "█ ▀ █", "▀   ▀"],
];

fn draw_splash(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let mut lines: Vec<Line> = vec![Line::raw("")];
    for row in 0..3 {
        let spans: Vec<Span> = LOGO
            .iter()
            .enumerate()
            // "zab" in the accent color, "term" in the secondary one.
            .flat_map(|(i, glyph)| [Span::styled(glyph[row], Style::new().fg(if i < 3 { t.accent } else { t.series[2] }).bold()), Span::raw(" ")])
            .collect();
        lines.push(Line::from(spans));
    }
    lines.push(Line::raw(""));
    lines.push(Line::styled("a terminal for Zabbix", Style::new().fg(t.muted)));
    lines.push(Line::raw(""));
    match &app.error {
        Some(err) => {
            lines.push(Line::styled(format!("✕  {}", truncate(err, 90)), Style::new().fg(t.err)));
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![Span::styled("config  ", Style::new().fg(t.muted)), Span::styled(tilde(&app.config_path), Style::new().fg(t.fg))]));
            lines.push(Line::styled("retrying on the next refresh · r to retry now", Style::new().fg(t.muted)));
        }
        None => lines.push(Line::from(vec![
            Span::styled(SPINNER[(app.frame % 10) as usize], Style::new().fg(t.accent)),
            Span::styled(format!("  connecting to {}", app.profile.api_url()), Style::new().fg(t.muted)),
        ])),
    }
    let h = lines.len() as u16;
    let rect = centered(area, area.width, h + 2);
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), rect);
}

fn draw_toasts(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let width = 48.min(area.width.saturating_sub(2));
    let mut y = area.y;
    for toast in &app.toasts {
        if y + 4 > area.y + area.height {
            break;
        }
        let (icon, color) = match toast.kind {
            ToastKind::Info => ("●", t.accent),
            ToastKind::Ok => ("✓", t.ok),
            ToastKind::Error => ("✕", t.err),
            ToastKind::Problem(s) => ("▲", t.sev[s as usize]),
        };
        let rect = Rect { x: area.x + area.width - width - 1, y, width, height: 4 };
        let block = Block::bordered().border_type(BorderType::Rounded).border_style(Style::new().fg(color));
        let body = Paragraph::new(vec![
            Line::from(vec![Span::styled(format!("{icon} "), Style::new().fg(color)), Span::styled(toast.title.clone(), Style::new().fg(t.fg).bold())]),
            Line::styled(truncate(&toast.body, width as usize - 4), Style::new().fg(t.muted)),
        ])
        .block(block);
        f.render_widget(Clear, rect);
        f.render_widget(body, rect);
        y += 4;
    }
}

fn modal_block<'a>(title: &str, app: &App) -> Block<'a> {
    let t = &app.theme;
    Block::bordered().border_type(BorderType::Rounded).border_style(Style::new().fg(t.accent)).title(Line::from(vec![
        Span::raw(" "),
        Span::styled(title.to_string(), Style::new().fg(t.accent).bold()),
        Span::raw(" "),
    ]))
}

fn draw_help(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let sections: [(&str, &[(&str, &str)]); 4] = [
        (
            "Global",
            &[("1 2 3 / tab", "switch view"), ("r", "refresh now"), ("t", "cycle theme"), ("o", "open in Zabbix web"), ("?", "this help"), ("q", "quit")],
        ),
        ("Lists", &[("j k ↑ ↓", "move"), ("g G", "top / bottom"), ("ctrl-d ctrl-u", "page"), ("/", "filter"), ("⏎", "drill into host")]),
        ("Problems", &[("a", "acknowledge (+ message, close)"), ("+ -", "minimum severity"), ("h", "hide acknowledged")]),
        ("Host", &[("[ ]", "time range 15m → 7d"), ("tab", "graphs / latest data"), ("esc", "back")]),
    ];
    let mut lines = vec![Line::raw("")];
    for (title, keys) in sections {
        lines.push(Line::styled(format!("  {title}"), Style::new().fg(t.fg).bold()));
        for (k, d) in keys {
            lines.push(Line::from(vec![Span::styled(format!("    {k:<16}"), Style::new().fg(t.accent)), Span::styled(*d, Style::new().fg(t.muted))]));
        }
        lines.push(Line::raw(""));
    }
    lines.push(Line::from(vec![
        Span::styled("  theme ", Style::new().fg(t.border)),
        Span::styled(t.name.clone(), Style::new().fg(t.muted)),
        Span::styled("   config ", Style::new().fg(t.border)),
        Span::styled(tilde(&app.config_path), Style::new().fg(t.muted)),
    ]));
    let rect = centered(area, 64, lines.len() as u16 + 2);
    f.render_widget(Clear, rect);
    f.render_widget(Paragraph::new(lines).block(modal_block("Keys", app)), rect);
}

fn draw_ack(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let Some(Modal::Ack { title, input, close, can_close, .. }) = &app.modal else {
        return;
    };
    let rect = centered(area, 70, 11);
    f.render_widget(Clear, rect);
    let block = modal_block("Acknowledge", app);
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let [head, _, field, opts, _, keys] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner.inner(ratatui::layout::Margin { horizontal: 1, vertical: 0 }));

    f.render_widget(Paragraph::new(Line::styled(title.clone(), Style::new().fg(t.fg))).wrap(Wrap { trim: true }), head);
    let input_block =
        Block::bordered().border_type(BorderType::Rounded).border_style(Style::new().fg(t.border)).title(Span::styled(" message ", Style::new().fg(t.muted)));
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(input.clone(), Style::new().fg(t.fg)), Span::styled("▏", Style::new().fg(t.accent))])).block(input_block),
        field,
    );
    let opt = if *can_close {
        let mark = if *close { "■" } else { "□" };
        Line::from(vec![
            Span::styled(format!("{mark} "), Style::new().fg(t.accent)),
            Span::styled("close problem too", Style::new().fg(t.muted)),
            Span::styled("  (tab)", Style::new().fg(t.border)),
        ])
    } else {
        Line::styled("trigger does not allow manual close", Style::new().fg(t.border))
    };
    f.render_widget(Paragraph::new(opt), opts);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("⏎", Style::new().fg(t.accent).bold()),
            Span::styled(" confirm   ", Style::new().fg(t.muted)),
            Span::styled("esc", Style::new().fg(t.accent).bold()),
            Span::styled(" cancel", Style::new().fg(t.muted)),
        ])),
        keys,
    );
}

fn tilde(path: &std::path::Path) -> String {
    let home = crate::config::home();
    match path.strip_prefix(&home) {
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}
