use super::fmt;
use super::widgets::*;
use crate::app::App;
use crate::model::{Availability, HostRow, Severity, Snapshot};
use crate::theme::Theme;
use chrono::Local;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Paragraph, Row, Table};

pub fn draw(f: &mut Frame, area: Rect, app: &mut App) {
    let Some(snap) = app.snapshot.as_ref() else {
        return;
    };
    let t = &app.theme;
    let host_rows = (snap.hosts.len() as u16 + 3).clamp(5, (area.height / 3).max(5));
    let [tiles, sev, middle, bottom] =
        Layout::vertical([Constraint::Length(7), Constraint::Length(4), Constraint::Min(8), Constraint::Length(host_rows)]).areas(area);

    draw_tiles(f, tiles, snap, t);
    draw_severity(f, sev, snap, t);
    let [chart, feed] = Layout::horizontal([Constraint::Percentage(62), Constraint::Percentage(38)]).areas(middle);
    draw_fleet_chart(f, chart, app);
    draw_feed(f, feed, app);
    draw_hosts(f, bottom, app);
}

fn tile(f: &mut Frame, area: Rect, title: &str, big: Vec<Line<'static>>, sub: Line<'static>, t: &Theme) {
    let mut lines = big;
    lines.push(Line::raw(""));
    lines.push(sub);
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center).block(panel(title, t)), area);
}

fn avg(values: impl Iterator<Item = f64>) -> Option<f64> {
    let v: Vec<f64> = values.collect();
    (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
}

fn peak<'a>(hosts: &[&'a HostRow], metric: fn(&HostRow) -> Option<f64>) -> Option<(&'a str, f64)> {
    hosts.iter().filter_map(|h| Some((h.name.as_str(), metric(h)?))).max_by(|a, b| a.1.total_cmp(&b.1))
}

/// Tile number: one decimal below 10 so small values don't read as 0.
fn short_pct(v: Option<f64>) -> String {
    match v {
        Some(v) if v < 10.0 => format!("{v:.1}"),
        Some(v) => format!("{v:.0}"),
        None => "-".into(),
    }
}

fn draw_tiles(f: &mut Frame, area: Rect, snap: &Snapshot, t: &Theme) {
    let cols = Layout::horizontal([Constraint::Ratio(1, 5); 5]).split(area);
    let monitored: Vec<&HostRow> = snap.hosts.iter().filter(|h| h.enabled).collect();
    let up = monitored.iter().filter(|h| h.availability == Availability::Up).count();
    let down = monitored.iter().filter(|h| h.availability == Availability::Down).count();
    let maint = monitored.iter().filter(|h| h.maintenance).count();
    let muted = Style::new().fg(t.muted);

    tile(
        f,
        cols[0],
        "Hosts",
        big_text(&snap.hosts.len().to_string(), "", t.fg, t),
        Line::from(vec![
            Span::styled("● ", Style::new().fg(t.ok)),
            Span::styled(format!("{up} up   "), muted),
            Span::styled("● ", Style::new().fg(if down > 0 { t.err } else { t.border })),
            Span::styled(format!("{down} down"), muted),
        ]),
        t,
    );

    let avail = if monitored.is_empty() { None } else { Some(up as f64 / monitored.len() as f64 * 100.0) };
    let avail_color = match avail {
        Some(a) if a >= 99.9 => t.ok,
        Some(a) if a >= 90.0 => t.warn,
        Some(_) => t.err,
        None => t.muted,
    };
    tile(
        f,
        cols[1],
        "Availability",
        big_text(&avail.map(|a| format!("{a:.0}")).unwrap_or("-".into()), "%", avail_color, t),
        if down > 0 {
            Line::styled(format!("{down} unreachable"), Style::new().fg(t.err))
        } else if maint > 0 {
            Line::styled(format!("◆ {maint} in maintenance"), muted)
        } else {
            Line::styled("agents reachable", muted)
        },
        t,
    );

    let active: Vec<_> = snap.problems.iter().filter(|p| !p.suppressed).collect();
    let unacked = active.iter().filter(|p| !p.acknowledged).count();
    let worst = active.iter().map(|p| p.severity).max();
    tile(
        f,
        cols[2],
        "Problems",
        big_text(&active.len().to_string(), "", worst.map(|s| t.sev[s as usize]).unwrap_or(t.ok), t),
        match worst {
            Some(s) if cols[2].width >= 32 => {
                Line::from(vec![Span::styled(format!("{unacked} unacked · worst "), muted), Span::styled(s.label(), Style::new().fg(t.sev[s as usize]))])
            }
            Some(s) => Line::from(vec![Span::styled(format!("{unacked} unacked · "), muted), Span::styled(s.short(), Style::new().fg(t.sev[s as usize]))]),
            None => Line::styled("all clear", Style::new().fg(t.ok)),
        },
        t,
    );

    // Unreachable hosts only have stale last values; keep them out of the averages.
    let live: Vec<&HostRow> = monitored.iter().copied().filter(|h| h.availability != Availability::Down).collect();
    for (col, title, metric) in [(cols[3], "Avg CPU", (|h: &HostRow| h.cpu) as fn(&HostRow) -> Option<f64>), (cols[4], "Avg Memory", |h: &HostRow| h.mem)] {
        let value = avg(live.iter().copied().filter_map(metric));
        let sub = match peak(&live, metric) {
            Some((name, v)) => Line::from(vec![
                Span::styled("peak ", muted),
                // Leave room for "peak " and " 100%" inside the borders.
                Span::styled(truncate(name, (col.width as usize).saturating_sub(13).max(4)), Style::new().fg(t.fg)),
                Span::styled(format!(" {}", fmt::pct(Some(v))), Style::new().fg(level(t, v))),
            ]),
            None => Line::styled("no data", muted),
        };
        tile(f, col, title, big_text(&short_pct(value), "%", value.map(|v| level(t, v)).unwrap_or(t.muted), t), sub, t);
    }
}

fn draw_severity(f: &mut Frame, area: Rect, snap: &Snapshot, t: &Theme) {
    let block = panel("Problems by severity", t);
    let inner = block.inner(area);
    f.render_widget(block, area);
    let [bar_area, legend_area] = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(inner);

    let mut counts = [0usize; 6];
    for p in snap.problems.iter().filter(|p| !p.suppressed) {
        counts[p.severity as usize] += 1;
    }
    let total: usize = counts.iter().sum();
    let width = bar_area.width as usize;

    let bar = if total == 0 {
        Line::styled("━".repeat(width), Style::new().fg(t.ok))
    } else {
        // Every non-zero severity gets at least one cell; the largest segment absorbs rounding.
        let mut cells: Vec<(usize, usize)> =
            Severity::ALL.iter().rev().map(|s| *s as usize).filter(|i| counts[*i] > 0).map(|i| (i, ((counts[i] * width) / total).max(1))).collect();
        let used: usize = cells.iter().map(|c| c.1).sum();
        if let Some(big) = cells.iter_mut().max_by_key(|c| c.1) {
            big.1 = (big.1 + width).saturating_sub(used);
        }
        Line::from(cells.into_iter().map(|(i, n)| Span::styled("█".repeat(n), Style::new().fg(t.sev[i]))).collect::<Vec<_>>())
    };
    f.render_widget(Paragraph::new(bar), bar_area);

    let mut legend = Vec::new();
    for s in Severity::ALL.iter().rev() {
        let n = counts[*s as usize];
        let color = t.sev[*s as usize];
        legend.push(Span::styled("■ ", Style::new().fg(if n > 0 { color } else { t.border })));
        legend.push(Span::styled(s.label(), Style::new().fg(if n > 0 { t.fg } else { t.muted })));
        legend.push(Span::styled(format!(" {n}     "), Style::new().fg(if n > 0 { color } else { t.muted }).bold()));
    }
    f.render_widget(Paragraph::new(Line::from(legend)), legend_area);
}

fn draw_fleet_chart(f: &mut Frame, area: Rect, app: &App) {
    let Some(snap) = &app.snapshot else { return };
    let t = &app.theme;
    type Metric = fn(&HostRow) -> (&[(f64, f64)], Option<f64>);
    let (label, metric): (&str, Metric) = if app.dash_mem { ("Memory", |h| (&h.mem_hist, h.mem)) } else { ("CPU", |h| (&h.cpu_hist, h.cpu)) };

    // The busiest six hosts; a chart with more lines than that is just noise.
    let mut hosts: Vec<&HostRow> = snap.hosts.iter().filter(|h| !metric(h).0.is_empty()).collect();
    hosts.sort_by(|a, b| metric(b).1.unwrap_or(0.0).total_cmp(&metric(a).1.unwrap_or(0.0)));
    hosts.truncate(6);

    let series: Vec<ChartSeries> = hosts
        .iter()
        .enumerate()
        .map(|(i, h)| ChartSeries { name: format!("{} {}", h.name, fmt::pct(metric(h).1)), points: metric(h).0, color: t.series[i % t.series.len()] })
        .collect();
    let now = Local::now().timestamp() as f64;
    let block = panel(format!("Fleet {label} · last 30m"), t).title_bottom(Line::from(vec![
        Span::styled(" m ", Style::new().fg(t.accent).bold()),
        Span::styled(if app.dash_mem { "show cpu " } else { "show memory " }, Style::new().fg(t.muted)),
    ]));
    time_chart(f, area, block, &series, [now - 1800.0, now], 5.0, YAxis::Percent, t);
}

fn draw_feed(f: &mut Frame, area: Rect, app: &App) {
    let Some(snap) = &app.snapshot else { return };
    let t = &app.theme;
    let block = panel("Latest problems", t);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut problems: Vec<_> = snap.problems.iter().filter(|p| !p.suppressed).collect();
    if problems.is_empty() {
        let lines = vec![
            Line::raw(""),
            Line::styled("✓", Style::new().fg(t.ok).bold()),
            Line::styled("All clear", Style::new().fg(t.fg).bold()),
            Line::styled("no active problems", Style::new().fg(t.muted)),
        ];
        let rect = Rect { y: inner.y + inner.height.saturating_sub(5) / 2, height: inner.height.min(5), ..inner };
        f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), rect);
        return;
    }
    problems.sort_by_key(|p| std::cmp::Reverse(p.clock));

    let w = inner.width as usize;
    let mut lines = Vec::new();
    for p in problems.iter().take((inner.height as usize).div_ceil(2)) {
        let ack = if p.acknowledged { Span::styled(" ✓", Style::new().fg(t.ok)) } else { Span::raw("") };
        lines.push(Line::from(vec![
            severity_badge(t, p.severity),
            Span::raw(" "),
            Span::styled(truncate(&p.host, w.saturating_sub(18)), Style::new().fg(t.fg).bold()),
            ack,
            Span::styled(format!("  {} ago", fmt::age(p.clock)), Style::new().fg(t.muted)),
        ]));
        lines.push(Line::from(vec![Span::styled("       ", Style::new()), Span::styled(truncate(&p.name, w.saturating_sub(8)), Style::new().fg(t.muted))]));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_hosts(f: &mut Frame, area: Rect, app: &mut App) {
    let hosts = app.hosts();
    let t = &app.theme;
    // Narrow terminals keep the essentials: status, CPU trend, memory, problems.
    let wide = area.width >= 130;
    let spark_w = if wide { 24usize } else { 14 };
    let rows: Vec<Row> = hosts
        .iter()
        .map(|h| {
            let mut cells = vec![
                Cell::from(status_dot(t, h)),
                Cell::from(host_name(t, h)),
                Cell::from(Line::from(vec![
                    sparkline(t, &h.cpu_hist, spark_w),
                    Span::styled(format!(" {:>5}", fmt::pct(h.cpu)), Style::new().fg(value_fg(t, h))),
                ])),
                Cell::from(Line::from([bar(t, h.mem, 10), vec![Span::styled(format!(" {:>5}", fmt::pct(h.mem)), Style::new().fg(value_fg(t, h)))]].concat())),
            ];
            if wide {
                cells.extend([
                    Cell::from(Line::from([bar(t, h.disk, 6), vec![Span::styled(format!(" {:>4}", fmt::pct(h.disk)), Style::new().fg(t.fg))]].concat())),
                    Cell::from(Span::styled(h.load.map(|l| format!("{l:.2}")).unwrap_or("-".into()), Style::new().fg(t.muted))),
                    Cell::from(Line::from(vec![
                        Span::styled("↓", Style::new().fg(t.series[1])),
                        Span::styled(format!("{:>9} ", h.net_in.map(fmt::bits).unwrap_or("-".into())), Style::new().fg(t.muted)),
                        Span::styled("↑", Style::new().fg(t.series[2])),
                        Span::styled(format!("{:>9}", h.net_out.map(fmt::bits).unwrap_or("-".into())), Style::new().fg(t.muted)),
                    ])),
                    Cell::from(Span::styled(h.uptime.map(|u| fmt::duration(u as i64)).unwrap_or("-".into()), Style::new().fg(t.muted))),
                ]);
            }
            cells.push(Cell::from(problem_counts(t, h)));
            Row::new(cells)
        })
        .collect();

    let mut header = vec!["", "Host", "CPU · 30m", "Memory"];
    let mut widths = vec![Constraint::Length(1), Constraint::Min(14), Constraint::Length(spark_w as u16 + 6), Constraint::Length(16)];
    if wide {
        header.extend(["Disk /", "Load", "Network", "Uptime"]);
        widths.extend([Constraint::Length(11), Constraint::Length(5), Constraint::Length(22), Constraint::Length(8)]);
    }
    header.push("Problems");
    widths.push(Constraint::Length(12));

    let table = Table::new(mark_selected(rows, app.hosts_state.selected(), t), widths)
        .header(Row::new(header).style(Style::new().fg(t.muted)))
        .column_spacing(2)
        .block(panel("Hosts", t));
    let mut state = app.hosts_state;
    f.render_stateful_widget(table, area, &mut state);
    app.hosts_state = state;
}
