//! Small reusable building blocks: panels, bars, badges, sparklines, charts
//! and the chunky 3-row digit font used on the dashboard tiles.

use crate::model::{Availability, HostRow, Severity};
use crate::theme::Theme;
use crate::ui::fmt;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Block, BorderType, Chart, Dataset, GraphType, LegendPosition, Paragraph, Row};

pub fn panel<'a>(title: impl Into<String>, t: &Theme) -> Block<'a> {
    Block::bordered().border_type(BorderType::Rounded).border_style(Style::new().fg(t.border)).title(Line::from(vec![
        Span::raw(" "),
        Span::styled(title.into(), Style::new().fg(t.fg).bold()),
        Span::raw(" "),
    ]))
}

pub fn level(t: &Theme, pct: f64) -> Color {
    if pct >= 90.0 {
        t.err
    } else if pct >= 70.0 {
        t.warn
    } else {
        t.ok
    }
}

/// `━━━━━━────` style meter for a percentage.
pub fn bar(t: &Theme, pct: Option<f64>, width: usize) -> Vec<Span<'static>> {
    let Some(p) = pct else {
        return vec![Span::styled("─".repeat(width), Style::new().fg(t.border))];
    };
    let filled = ((p.clamp(0.0, 100.0) / 100.0) * width as f64).round() as usize;
    let filled = if p > 0.0 { filled.max(1) } else { 0 };
    vec![Span::styled("━".repeat(filled), Style::new().fg(level(t, p))), Span::styled("─".repeat(width - filled.min(width)), Style::new().fg(t.border))]
}

pub fn severity_badge(t: &Theme, s: Severity) -> Span<'static> {
    Span::styled(format!(" {:<4} ", s.short()), Style::new().bg(t.sev[s as usize]).fg(t.on_badge()).add_modifier(Modifier::BOLD))
}

pub fn pill(label: impl Into<String>, fg: Color, bg: Color) -> Span<'static> {
    Span::styled(format!(" {} ", label.into()), Style::new().fg(fg).bg(bg))
}

pub fn status_dot(t: &Theme, h: &HostRow) -> Span<'static> {
    if !h.enabled {
        return Span::styled("✕", Style::new().fg(t.muted));
    }
    if h.maintenance {
        return Span::styled("◆", Style::new().fg(t.info));
    }
    match h.availability {
        Availability::Up => Span::styled("●", Style::new().fg(t.ok)),
        Availability::Down => Span::styled("●", Style::new().fg(t.err)),
        Availability::Unknown => Span::styled("○", Style::new().fg(t.muted)),
    }
}

/// Host name styled by state: struck through when disabled, red when the
/// agent is unreachable (its metrics are stale then).
pub fn host_name(t: &Theme, h: &HostRow) -> Span<'static> {
    let style = match (h.enabled, h.availability) {
        (false, _) => Style::new().fg(t.muted).crossed_out(),
        (_, Availability::Down) => Style::new().fg(t.err).bold(),
        _ => Style::new().fg(t.fg).bold(),
    };
    Span::styled(h.name.clone(), style)
}

/// Text color for a host's metric values; muted when they are stale.
pub fn value_fg(t: &Theme, h: &HostRow) -> Color {
    if h.availability == Availability::Down || !h.enabled { t.muted } else { t.fg }
}

/// Per-severity problem counters, e.g. `2 1` in their severity colors.
pub fn problem_counts(t: &Theme, h: &HostRow) -> Line<'static> {
    if h.problem_count() == 0 {
        return Line::from(Span::styled("✓", Style::new().fg(t.ok)));
    }
    let mut spans = Vec::new();
    for s in Severity::ALL.iter().rev() {
        let n = h.problems[*s as usize];
        if n > 0 {
            spans.push(Span::styled(format!(" {n} "), Style::new().bg(t.sev[*s as usize]).fg(t.on_badge()).bold()));
            spans.push(Span::raw(" "));
        }
    }
    Line::from(spans)
}

const TICKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Inline sparkline of the most recent `width` samples. Auto-scales with a
/// floor so an idle host looks idle instead of noisy.
pub fn sparkline(t: &Theme, points: &[(f64, f64)], width: usize) -> Span<'static> {
    if points.is_empty() {
        return Span::styled("·".repeat(width), Style::new().fg(t.border));
    }
    let tail: Vec<f64> = points.iter().rev().take(width).rev().map(|p| p.1).collect();
    // Scale to the window's own range (at least 8 points wide) so the shape of
    // the trend shows, whether a host idles at 3% or runs hot at 90%.
    let lo = tail.iter().cloned().fold(f64::INFINITY, f64::min);
    let hi = tail.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let span = (hi - lo).max(8.0);
    let base = (lo - (span - (hi - lo)) / 2.0).max(0.0);
    let mut s: String = tail.iter().map(|v| TICKS[(((v - base) / span) * 7.0).round().clamp(0.0, 7.0) as usize]).collect();
    if tail.len() < width {
        s = format!("{}{s}", " ".repeat(width - tail.len()));
    }
    let last = tail.last().copied().unwrap_or(0.0);
    Span::styled(s, Style::new().fg(if last >= 70.0 { level(t, last) } else { t.accent }))
}

const DIGITS: [[&str; 3]; 10] = [
    ["█▀█", "█ █", "▀▀▀"],
    ["▀█ ", " █ ", "▀▀▀"],
    ["▀▀█", "█▀▀", "▀▀▀"],
    ["▀▀█", " ▀█", "▀▀▀"],
    ["█ █", "▀▀█", "  ▀"],
    ["█▀▀", "▀▀█", "▀▀▀"],
    ["█▀▀", "█▀█", "▀▀▀"],
    ["▀▀█", "  █", "  ▀"],
    ["█▀█", "█▀█", "▀▀▀"],
    ["█▀█", "▀▀█", "▀▀▀"],
];

/// Render `text` (digits, '.', '-') in the 3-row block font. `suffix` is
/// drawn small next to the baseline, e.g. a `%` sign.
pub fn big_text(text: &str, suffix: &str, color: Color, t: &Theme) -> Vec<Line<'static>> {
    let mut rows = [String::new(), String::new(), String::new()];
    for (i, ch) in text.chars().enumerate() {
        let glyph: [&str; 3] = match ch {
            '0'..='9' => DIGITS[ch as usize - '0' as usize],
            '.' => [" ", " ", "▀"],
            '-' => ["   ", "▀▀▀", "   "],
            _ => [" ", " ", " "],
        };
        for r in 0..3 {
            if i > 0 {
                rows[r].push(' ');
            }
            rows[r].push_str(glyph[r]);
        }
    }
    let pad = " ".repeat(suffix.chars().count() + 1);
    rows.into_iter()
        .enumerate()
        .map(|(r, row)| {
            let tail = if r == 2 { format!(" {suffix}") } else { pad.clone() };
            // Leading pad balances the suffix so the digits stay centered.
            Line::from(vec![Span::raw(pad.clone()), Span::styled(row, Style::new().fg(color).bold()), Span::styled(tail, Style::new().fg(t.muted))])
        })
        .collect()
}

pub struct ChartSeries<'a> {
    pub name: String,
    pub points: &'a [(f64, f64)],
    pub color: Color,
}

/// Braille line chart with time on X. `floor` is the minimum Y range so that
/// flat, near-zero series don't get stretched into noise.
#[allow(clippy::too_many_arguments)]
pub fn time_chart(f: &mut Frame, area: Rect, block: Block, series: &[ChartSeries], x: [f64; 2], floor: f64, y: YAxis, t: &Theme) {
    let has_data = series.iter().any(|s| !s.points.is_empty());
    if !has_data {
        let msg = Paragraph::new(vec![Line::raw(""), Line::styled("no data yet", Style::new().fg(t.muted))]).alignment(Alignment::Center).block(block);
        f.render_widget(msg, area);
        return;
    }
    let max = series.iter().flat_map(|s| s.points.iter().map(|p| p.1)).fold(0.0_f64, f64::max);
    let mut top = nice_ceil((max * 1.15).max(floor));
    // Percent charts never need headroom above 100%.
    if y == YAxis::Percent && max <= 100.0 {
        top = top.min(100.0);
    }
    let datasets: Vec<Dataset> = series
        .iter()
        .map(|s| Dataset::default().name(s.name.clone()).marker(Marker::Braille).graph_type(GraphType::Line).style(Style::new().fg(s.color)).data(s.points))
        .collect();
    let axis_style = Style::new().fg(t.muted);
    let chart = Chart::new(datasets)
        .block(block)
        .x_axis(Axis::default().style(Style::new().fg(t.border)).bounds(x).labels(vec![
            Span::styled(fmt::hhmm(x[0]), axis_style),
            Span::styled(fmt::hhmm((x[0] + x[1]) / 2.0), axis_style),
            Span::styled("now", axis_style),
        ]))
        .y_axis(Axis::default().style(Style::new().fg(t.border)).bounds([0.0, top]).labels(vec![
            Span::styled(y.label(0.0), axis_style),
            Span::styled(y.label(top / 2.0), axis_style),
            Span::styled(y.label(top), axis_style),
        ]))
        .legend_position(if series.len() > 1 { Some(LegendPosition::TopRight) } else { None })
        .hidden_legend_constraints((ratatui::layout::Constraint::Percentage(60), ratatui::layout::Constraint::Percentage(60)));
    f.render_widget(chart, area);
}

/// Round up to 1/2/2.5/5 x 10^n so axis labels stay readable.
fn nice_ceil(v: f64) -> f64 {
    if v <= 0.0 {
        return 1.0;
    }
    let mag = 10f64.powf(v.log10().floor());
    let n = v / mag;
    let step = [1.0, 2.0, 2.5, 5.0, 10.0].into_iter().find(|s| n <= *s).unwrap_or(10.0);
    step * mag
}

/// Highlight the selected row through its base style rather than
/// `row_highlight_style`, which is painted over the cells and would wipe out
/// the background of severity badges and problem counters.
pub fn mark_selected<'a>(rows: Vec<Row<'a>>, selected: Option<usize>, t: &Theme) -> Vec<Row<'a>> {
    rows.into_iter().enumerate().map(|(i, row)| if Some(i) == selected { row.style(Style::new().bg(t.surface)) } else { row }).collect()
}

pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width.saturating_sub(2));
    let h = height.min(area.height.saturating_sub(2));
    Rect { x: area.x + (area.width - w) / 2, y: area.y + (area.height - h) / 2, width: w, height: h }
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum YAxis {
    Percent,
    Bits,
    Plain,
}

impl YAxis {
    fn label(self, v: f64) -> String {
        match self {
            Self::Percent if v < 10.0 && v.fract() != 0.0 => format!("{v:.1}%"),
            Self::Percent => format!("{v:.0}%"),
            Self::Bits => fmt::bits(v),
            Self::Plain => fmt::number((v * 100.0).round() / 100.0),
        }
    }
}
