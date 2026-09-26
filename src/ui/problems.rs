use super::fmt;
use super::widgets::*;
use crate::app::App;
use crate::model::Severity;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Table, Wrap};

pub fn draw(f: &mut Frame, area: Rect, app: &mut App) {
    let [chips, table_area, detail] = Layout::vertical([Constraint::Length(1), Constraint::Min(6), Constraint::Length(8)]).areas(area);
    draw_chips(f, chips, app);
    draw_table(f, table_area, app);
    draw_detail(f, detail, app);
}

fn draw_chips(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let total = app.snapshot.as_ref().map(|s| s.problems.len()).unwrap_or(0);
    let mut spans = vec![
        Span::styled(format!(" {}", app.problems().len()), Style::new().fg(t.fg).bold()),
        Span::styled(format!(" of {total} problems"), Style::new().fg(t.muted)),
        Span::styled("   severity ≥ ", Style::new().fg(t.border)),
    ];
    // A mini severity scale showing where the threshold sits.
    for s in Severity::ALL {
        let on = s >= app.min_severity;
        spans.push(Span::styled("■", Style::new().fg(if on { t.sev[s as usize] } else { t.border })));
    }
    spans.push(Span::styled(format!(" {}", app.min_severity.label()), Style::new().fg(t.fg)));
    spans.push(Span::styled("   acked ", Style::new().fg(t.border)));
    spans.push(pill(if app.hide_acked { "hidden" } else { "shown" }, t.fg, t.surface));
    if !app.problem_filter.is_empty() {
        spans.push(Span::styled("   filter ", Style::new().fg(t.border)));
        spans.push(pill(app.problem_filter.clone(), t.on_badge(), t.accent));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_table(f: &mut Frame, area: Rect, app: &mut App) {
    let problems = app.problems();
    let t = &app.theme;
    if problems.is_empty() {
        let lines = vec![
            Line::raw(""),
            Line::raw(""),
            Line::styled("✓", Style::new().fg(t.ok).bold()),
            Line::styled("Nothing to see here", Style::new().fg(t.fg).bold()),
            Line::styled("no problems match the current filters", Style::new().fg(t.muted)),
        ];
        f.render_widget(Paragraph::new(lines).alignment(Alignment::Center).block(panel("Problems", t)), area);
        return;
    }
    let rows: Vec<Row> = problems
        .iter()
        .map(|p| {
            let dim = p.suppressed || p.acknowledged;
            let text = if dim { t.muted } else { t.fg };
            Row::new(vec![
                Cell::from(severity_badge(t, p.severity)),
                Cell::from(Span::styled(fmt::clock(p.clock), Style::new().fg(t.muted))),
                Cell::from(Span::styled(fmt::age(p.clock), Style::new().fg(t.fg))),
                Cell::from(Span::styled(p.host.clone(), Style::new().fg(text).bold())),
                Cell::from(Span::styled(p.name.clone(), Style::new().fg(text))),
                Cell::from(if p.acknowledged { Span::styled("✓", Style::new().fg(t.ok)) } else { Span::styled("·", Style::new().fg(t.border)) }),
                Cell::from(Span::styled(p.tags.join(" "), Style::new().fg(t.border))),
            ])
        })
        .collect();
    let header = Row::new(["Severity", "Time", "Age", "Host", "Problem", "Ack", "Tags"]).style(Style::new().fg(t.muted)).bottom_margin(1);
    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(7),
            Constraint::Length(18),
            Constraint::Min(30),
            Constraint::Length(3),
            Constraint::Min(10),
        ],
    )
    .header(header)
    .column_spacing(2)
    .row_highlight_style(Style::new().bg(t.surface))
    .highlight_symbol(Span::styled("▌", Style::new().fg(t.accent)))
    .block(panel("Problems", t));
    let mut state = app.problems_state;
    f.render_stateful_widget(table, area, &mut state);
    app.problems_state = state;
}

fn draw_detail(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let Some(p) = app.selected_problem() else {
        f.render_widget(panel("Details", t), area);
        return;
    };
    let color = t.sev[p.severity as usize];
    let kv = |k: &str, v: String| vec![Span::styled(format!("{k} "), Style::new().fg(t.muted)), Span::styled(v, Style::new().fg(t.fg)), Span::raw("    ")];
    let mut lines = vec![
        Line::from(vec![Span::styled("▲ ", Style::new().fg(color)), Span::styled(p.name.clone(), Style::new().fg(t.fg).bold())]),
        Line::raw(""),
        Line::from(
            [
                kv("Host", p.host.clone()),
                kv("Severity", p.severity.label().into()),
                kv("Started", format!("{} ({} ago)", fmt::clock(p.clock), fmt::age(p.clock))),
                kv("Event", format!("#{}", p.eventid)),
            ]
            .concat(),
        ),
    ];
    let mut status = vec![Span::styled("Status ", Style::new().fg(t.muted))];
    status.push(if p.acknowledged { Span::styled("acknowledged", Style::new().fg(t.ok)) } else { Span::styled("unacknowledged", Style::new().fg(t.warn)) });
    if p.suppressed {
        status.push(Span::styled("  suppressed", Style::new().fg(t.info)));
    }
    if !p.opdata.is_empty() {
        status.push(Span::styled("    Opdata ", Style::new().fg(t.muted)));
        status.push(Span::styled(p.opdata.clone(), Style::new().fg(t.fg)));
    }
    lines.push(Line::from(status));
    if !p.tags.is_empty() {
        let mut tags = vec![Span::styled("Tags   ", Style::new().fg(t.muted))];
        for tag in &p.tags {
            tags.push(pill(tag.clone(), t.fg, t.surface));
            tags.push(Span::raw(" "));
        }
        lines.push(Line::from(tags));
    }
    let block = panel("Details", t).border_style(Style::new().fg(color)).title_bottom(Line::from(vec![
        Span::styled(" a ", Style::new().fg(t.accent).bold()),
        Span::styled("acknowledge  ", Style::new().fg(t.muted)),
        Span::styled("⏎ ", Style::new().fg(t.accent).bold()),
        Span::styled("host  ", Style::new().fg(t.muted)),
        Span::styled("o ", Style::new().fg(t.accent).bold()),
        Span::styled("web ", Style::new().fg(t.muted)),
    ]));
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }).block(block), area);
}
