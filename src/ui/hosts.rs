use super::fmt;
use super::widgets::*;
use crate::app::App;
use crate::model::Availability;
use chrono::Local;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Table};

pub fn draw(f: &mut Frame, area: Rect, app: &mut App) {
    // The table only takes the rows it needs; the preview charts get the rest.
    let rows = app.hosts().len() as u16 + 4;
    let table_h = rows.clamp(6, area.height.saturating_sub(12).max(6));
    let [info, table_area, preview] = Layout::vertical([Constraint::Length(1), Constraint::Length(table_h), Constraint::Min(11)]).areas(area);
    draw_info(f, info, app);
    draw_table(f, table_area, app);
    draw_preview(f, preview, app);
}

fn draw_info(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let total = app.snapshot.as_ref().map(|s| s.hosts.len()).unwrap_or(0);
    let shown = app.hosts().len();
    let mut spans = vec![
        Span::styled(format!(" {shown}"), Style::new().fg(t.fg).bold()),
        Span::styled(format!(" of {total} hosts"), Style::new().fg(t.muted)),
        Span::styled("   sort ", Style::new().fg(t.border)),
        pill(app.host_sort.label(), t.fg, t.surface),
    ];
    if !app.host_filter.is_empty() {
        spans.push(Span::styled("   filter ", Style::new().fg(t.border)));
        spans.push(pill(app.host_filter.clone(), t.on_badge(), t.accent));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_table(f: &mut Frame, area: Rect, app: &mut App) {
    let hosts = app.hosts();
    let t = &app.theme;
    let rows: Vec<Row> = hosts
        .iter()
        .map(|h| {
            Row::new(vec![
                Cell::from(status_dot(t, h)),
                Cell::from(host_name(t, h)),
                Cell::from(Span::styled(h.groups.join(", "), Style::new().fg(t.muted))),
                Cell::from(Span::styled(h.address.clone(), Style::new().fg(t.muted))),
                Cell::from(Line::from([bar(t, h.cpu, 8), vec![Span::styled(format!(" {:>5}", fmt::pct(h.cpu)), Style::new().fg(value_fg(t, h)))]].concat())),
                Cell::from(Line::from([bar(t, h.mem, 8), vec![Span::styled(format!(" {:>5}", fmt::pct(h.mem)), Style::new().fg(value_fg(t, h)))]].concat())),
                Cell::from(Span::styled(fmt::pct(h.disk), Style::new().fg(h.disk.map(|d| level(t, d)).unwrap_or(t.muted)))),
                Cell::from(Span::styled(h.load.map(|l| format!("{l:.2}")).unwrap_or("-".into()), Style::new().fg(t.muted))),
                Cell::from(Span::styled(h.uptime.map(|u| fmt::duration(u as i64)).unwrap_or("-".into()), Style::new().fg(t.muted))),
                Cell::from(problem_counts(t, h)),
            ])
        })
        .collect();
    let header =
        Row::new(["", "Name", "Groups", "Interface", "CPU", "Memory", "Disk", "Load", "Uptime", "Problems"]).style(Style::new().fg(t.muted)).bottom_margin(1);
    let table = Table::new(
        mark_selected(rows, app.hosts_state.selected(), t),
        [
            Constraint::Length(1),
            Constraint::Min(16),
            Constraint::Min(14),
            Constraint::Length(22),
            Constraint::Length(14),
            Constraint::Length(14),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(8),
            Constraint::Length(12),
        ],
    )
    .header(header)
    .column_spacing(2)
    .highlight_symbol(Span::styled("▌", Style::new().fg(t.accent)))
    .block(panel("Hosts", t));
    let mut state = app.hosts_state;
    f.render_stateful_widget(table, area, &mut state);
    app.hosts_state = state;
}

fn draw_preview(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let hosts = app.hosts();
    let Some(h) = hosts.get(app.hosts_state.selected().unwrap_or(0)) else {
        return;
    };
    let [cpu, mem, info] = Layout::horizontal([Constraint::Percentage(37), Constraint::Percentage(37), Constraint::Percentage(26)]).areas(area);

    let now = Local::now().timestamp() as f64;
    let x = [now - 1800.0, now];
    time_chart(
        f,
        cpu,
        panel(format!("CPU  {}", fmt::pct(h.cpu)), t),
        &[ChartSeries { name: "cpu".into(), points: &h.cpu_hist, color: t.series[0] }],
        x,
        5.0,
        YAxis::Percent,
        t,
    );
    time_chart(
        f,
        mem,
        panel(format!("Memory  {}", fmt::pct(h.mem)), t),
        &[ChartSeries { name: "mem".into(), points: &h.mem_hist, color: t.series[2] }],
        x,
        5.0,
        YAxis::Percent,
        t,
    );

    let (state, color) = match (h.enabled, h.maintenance, h.availability) {
        (false, _, _) => ("disabled", t.muted),
        (_, true, _) => ("maintenance", t.info),
        (_, _, Availability::Up) => ("available", t.ok),
        (_, _, Availability::Down) => ("unreachable", t.err),
        _ => ("unknown", t.muted),
    };
    let kv = |k: &str, v: String| Line::from(vec![Span::styled(format!("{k:<8}"), Style::new().fg(t.muted)), Span::styled(v, Style::new().fg(t.fg))]);
    let mut lines = vec![
        Line::from(vec![status_dot(t, h), Span::raw(" "), Span::styled(state, Style::new().fg(color).bold())]),
        Line::raw(""),
        kv("CPUs", h.cpus.map(|c| format!("{c:.0}")).unwrap_or("-".into())),
        kv("Net ↓", h.net_in.map(fmt::bits).unwrap_or("-".into())),
        kv("Net ↑", h.net_out.map(fmt::bits).unwrap_or("-".into())),
        kv("Uptime", h.uptime.map(|u| fmt::duration(u as i64)).unwrap_or("-".into())),
    ];
    if !h.error.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::styled(truncate(&h.error, info.width.saturating_sub(4) as usize), Style::new().fg(t.err)));
    }
    f.render_widget(Paragraph::new(lines).block(panel(truncate(&h.name, 24), t).title_bottom(Line::styled(" ⏎ details ", Style::new().fg(t.muted)))), info);
}
