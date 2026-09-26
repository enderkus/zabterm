use super::fmt;
use super::widgets::*;
use crate::app::{App, HostTab, RANGES};
use crate::model::{Availability, HostDetail};
use chrono::{Local, TimeZone};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Table};

pub fn draw(f: &mut Frame, area: Rect, app: &mut App) {
    let [head, body] = Layout::vertical([Constraint::Length(5), Constraint::Min(0)]).areas(area);
    draw_head(f, head, app);

    let Some(detail) = app.detail.clone() else {
        let t = &app.theme;
        let msg = Paragraph::new(vec![Line::raw(""), Line::styled("loading history…", Style::new().fg(t.muted))]).alignment(Alignment::Center);
        f.render_widget(msg, body);
        return;
    };
    match app.host_tab {
        HostTab::Graphs => draw_graphs(f, body, app, &detail),
        HostTab::Items => draw_items(f, body, app),
    }
}

fn draw_head(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let Some(h) = app.detail_row() else { return };
    let block = panel("Host", t);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let (state, color) = match (h.enabled, h.maintenance, h.availability) {
        (false, _, _) => ("disabled", t.muted),
        (_, true, _) => ("in maintenance", t.info),
        (_, _, Availability::Up) => ("available", t.ok),
        (_, _, Availability::Down) => ("unreachable", t.err),
        _ => ("unknown", t.muted),
    };
    let sep = || Span::styled("  ·  ", Style::new().fg(t.border));
    let line1 = Line::from(vec![
        status_dot(t, h),
        Span::raw(" "),
        Span::styled(h.name.clone(), Style::new().fg(t.fg).bold()),
        Span::raw("  "),
        pill(state, t.on_badge(), color),
        sep(),
        Span::styled(h.address.clone(), Style::new().fg(t.muted)),
        sep(),
        Span::styled(h.groups.join(", "), Style::new().fg(t.muted)),
    ]);
    let mut facts = Vec::new();
    for (k, v) in app.detail.as_ref().map(|d| d.facts.clone()).unwrap_or_default() {
        facts.push(Span::styled(format!("{k} "), Style::new().fg(t.muted)));
        facts.push(Span::styled(truncate(&v, 48), Style::new().fg(t.fg)));
        facts.push(Span::raw("    "));
    }

    let tab = |label: &'static str, on: bool| {
        if on {
            Span::styled(format!(" {label} "), Style::new().fg(t.on_badge()).bg(t.accent).bold())
        } else {
            Span::styled(format!(" {label} "), Style::new().fg(t.muted))
        }
    };
    let mut tabs =
        vec![tab("Graphs", app.host_tab == HostTab::Graphs), Span::raw(" "), tab("Latest data", app.host_tab == HostTab::Items), Span::raw("      ")];
    tabs.push(Span::styled("range ", Style::new().fg(t.muted)));
    for (i, (_, label)) in RANGES.iter().enumerate() {
        let style = if i == app.range_idx { Style::new().fg(t.accent).bold().underlined() } else { Style::new().fg(t.border) };
        tabs.push(Span::styled(*label, style));
        tabs.push(Span::raw(" "));
    }

    let [l1, l2, l3] = Layout::vertical([Constraint::Length(1); 3]).areas(inner);
    f.render_widget(Paragraph::new(line1), l1);
    f.render_widget(Paragraph::new(Line::from(facts)), l2);
    f.render_widget(Paragraph::new(Line::from(tabs)), l3);
    f.render_widget(Paragraph::new(problem_counts(t, h)).alignment(Alignment::Right), l1);
}

fn last(points: &[(f64, f64)]) -> Option<f64> {
    points.last().map(|p| p.1)
}

fn draw_graphs(f: &mut Frame, area: Rect, app: &App, d: &HostDetail) {
    let t = &app.theme;
    let [charts, side] = Layout::horizontal([Constraint::Min(40), Constraint::Length(36)]).areas(area);
    let [top, bottom] = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(charts);
    let [cpu, mem] = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(top);
    let [net, load] = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(bottom);

    let now = Local::now().timestamp() as f64;
    let x = [now - d.range as f64, now];
    let r = app.range_label();

    time_chart(
        f,
        cpu,
        panel(format!("CPU  {}  · {r}", fmt::pct(last(&d.cpu))), t),
        &[ChartSeries { name: "cpu".into(), points: &d.cpu, color: t.series[0] }],
        x,
        5.0,
        YAxis::Percent,
        t,
    );
    time_chart(
        f,
        mem,
        panel(format!("Memory  {}  · {r}", fmt::pct(last(&d.mem))), t),
        &[ChartSeries { name: "mem".into(), points: &d.mem, color: t.series[2] }],
        x,
        5.0,
        YAxis::Percent,
        t,
    );
    let net_title =
        format!("Network  ↓ {}  ↑ {}  · {r}", last(&d.net_in).map(fmt::bits).unwrap_or("-".into()), last(&d.net_out).map(fmt::bits).unwrap_or("-".into()));
    time_chart(
        f,
        net,
        panel(net_title, t),
        &[ChartSeries { name: "in".into(), points: &d.net_in, color: t.series[1] }, ChartSeries { name: "out".into(), points: &d.net_out, color: t.series[3] }],
        x,
        1000.0,
        YAxis::Bits,
        t,
    );
    time_chart(
        f,
        load,
        panel(format!("Load 1m  {}  · {r}", last(&d.load).map(|v| format!("{v:.2}")).unwrap_or("-".into())), t),
        &[ChartSeries { name: "load".into(), points: &d.load, color: t.series[4] }],
        x,
        1.0,
        YAxis::Plain,
        t,
    );

    let disks_h = (d.disks.len() as u16 * 2 + 2).clamp(4, side.height / 2);
    let [disks, problems] = Layout::vertical([Constraint::Length(disks_h), Constraint::Min(4)]).areas(side);
    let w = disks.width.saturating_sub(4) as usize;
    let mut lines = Vec::new();
    for (mount, used) in &d.disks {
        let label = truncate(mount, w.saturating_sub(7));
        lines.push(Line::from(vec![
            Span::styled(format!("{label:<width$}", width = w.saturating_sub(6)), Style::new().fg(t.fg)),
            Span::styled(format!("{:>6}", fmt::pct(Some(*used))), Style::new().fg(level(t, *used))),
        ]));
        lines.push(Line::from(bar(t, Some(*used), w)));
    }
    if lines.is_empty() {
        lines.push(Line::styled("no filesystems", Style::new().fg(t.muted)));
    }
    f.render_widget(Paragraph::new(lines).block(panel("Filesystems", t)), disks);

    let host_problems: Vec<_> = app.snapshot.as_ref().map(|s| s.problems.iter().filter(|p| p.hostid == d.hostid).collect()).unwrap_or_default();
    let mut lines = Vec::new();
    for p in &host_problems {
        lines.push(Line::from(vec![severity_badge(t, p.severity), Span::styled(format!(" {} ago", fmt::age(p.clock)), Style::new().fg(t.muted))]));
        lines.push(Line::styled(truncate(&p.name, w), Style::new().fg(t.fg)));
    }
    if lines.is_empty() {
        lines.push(Line::styled("✓ no active problems", Style::new().fg(t.ok)));
    }
    f.render_widget(Paragraph::new(lines).block(panel(format!("Problems {}", host_problems.len()), t)), problems);
}

fn draw_items(f: &mut Frame, area: Rect, app: &mut App) {
    let items = app.items();
    let t = &app.theme;
    let total = app.detail.as_ref().map(|d| d.items.len()).unwrap_or(0);
    let rows: Vec<Row> = items
        .iter()
        .map(|i| {
            let updated = i.lastclock.parse::<i64>().ok().filter(|c| *c > 0);
            Row::new(vec![
                Cell::from(Span::styled(i.name.clone(), Style::new().fg(t.fg))),
                Cell::from(Span::styled(i.key_.clone(), Style::new().fg(t.border))),
                Cell::from(Span::styled(fmt::item_value(i), Style::new().fg(if i.is_numeric() { t.accent } else { t.fg }).bold())),
                Cell::from(Span::styled(updated.map(|c| format!("{} ago", fmt::age(c))).unwrap_or("never".into()), Style::new().fg(t.muted))),
                Cell::from(Span::styled(
                    updated.and_then(|c| Local.timestamp_opt(c, 0).single()).map(|d| d.format("%H:%M:%S").to_string()).unwrap_or_default(),
                    Style::new().fg(t.border),
                )),
            ])
        })
        .collect();
    let mut title = format!("Latest data  {} of {total}", items.len());
    if !app.item_filter.is_empty() {
        title.push_str(&format!("  · /{}", app.item_filter));
    }
    let header = Row::new(["Name", "Key", "Last value", "Updated", ""]).style(Style::new().fg(t.muted)).bottom_margin(1);
    let table = Table::new(
        mark_selected(rows, app.items_state.selected(), t),
        [Constraint::Percentage(34), Constraint::Percentage(30), Constraint::Percentage(20), Constraint::Length(10), Constraint::Length(8)],
    )
    .header(header)
    .column_spacing(2)
    .highlight_symbol(Span::styled("▌", Style::new().fg(t.accent)))
    .block(panel(title, t));
    let mut state = app.items_state;
    f.render_stateful_widget(table, area, &mut state);
    app.items_state = state;
}
