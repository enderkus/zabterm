//! Colors. Built-in palettes mirror the popular Omarchy themes, and `auto`
//! reads whatever Omarchy theme is active right now, so zabterm blends in with
//! the rest of the desktop and follows along when the user switches themes.

use crate::config::home;
use ratatui::style::Color;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct Palette {
    bg: Rgb,
    fg: Rgb,
    red: Rgb,
    green: Rgb,
    yellow: Rgb,
    blue: Rgb,
    magenta: Rgb,
    cyan: Rgb,
    bright_black: Rgb,
    accent: Rgb,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub bg: Color,
    pub fg: Color,
    pub muted: Color,
    pub border: Color,
    pub surface: Color,
    pub accent: Color,
    pub ok: Color,
    pub warn: Color,
    pub err: Color,
    pub info: Color,
    pub sev: [Color; 6],
    pub series: [Color; 6],
}

pub const BUILTIN: [&str; 8] = ["tokyo-night", "catppuccin", "gruvbox", "nord", "everforest", "rose-pine", "kanagawa", "matte-black"];

#[derive(Debug, Clone, Copy, PartialEq)]
struct Rgb(u8, u8, u8);

impl Rgb {
    fn hex(s: &str) -> Option<Self> {
        let s = s.trim().trim_matches(|c| c == '"' || c == '\'').trim_start_matches('#').trim_start_matches("0x");
        if s.len() < 6 {
            return None;
        }
        let n = u32::from_str_radix(&s[..6], 16).ok()?;
        Some(Rgb((n >> 16) as u8, (n >> 8) as u8, n as u8))
    }

    fn mix(self, other: Rgb, t: f32) -> Rgb {
        let l = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Rgb(l(self.0, other.0), l(self.1, other.1), l(self.2, other.2))
    }

    fn color(self) -> Color {
        if truecolor() { Color::Rgb(self.0, self.1, self.2) } else { Color::Indexed(self.ansi256()) }
    }

    /// Nearest entry in the xterm 256-color palette (6x6x6 cube or gray ramp).
    fn ansi256(self) -> u8 {
        const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
        let idx = |v: u8| LEVELS.iter().enumerate().min_by_key(|(_, l)| (v as i32 - **l as i32).abs()).map(|(i, _)| i).unwrap_or(0);
        let (r, g, b) = (idx(self.0), idx(self.1), idx(self.2));
        let cube = Rgb(LEVELS[r], LEVELS[g], LEVELS[b]);
        let avg = (self.0 as u32 + self.1 as u32 + self.2 as u32) / 3;
        let gray_i = ((avg.saturating_sub(8)) / 10).min(23) as u8;
        let gray_v = 8 + gray_i * 10;
        let gray = Rgb(gray_v, gray_v, gray_v);
        let dist = |c: Rgb| (0..3).map(|i| [c.0, c.1, c.2][i] as i32 - [self.0, self.1, self.2][i] as i32).map(|d| d * d).sum::<i32>();
        if dist(gray) < dist(cube) { 232 + gray_i } else { 16 + 36 * r as u8 + 6 * g as u8 + b as u8 }
    }
}

impl Palette {
    #[allow(clippy::too_many_arguments)]
    fn new(bg: &str, fg: &str, red: &str, green: &str, yellow: &str, blue: &str, magenta: &str, cyan: &str, bright_black: &str, accent: &str) -> Self {
        let h = |s| Rgb::hex(s).expect("valid built-in hex");
        Self {
            bg: h(bg),
            fg: h(fg),
            red: h(red),
            green: h(green),
            yellow: h(yellow),
            blue: h(blue),
            magenta: h(magenta),
            cyan: h(cyan),
            bright_black: h(bright_black),
            accent: h(accent),
        }
    }

    fn builtin(name: &str) -> Option<Self> {
        Some(match name {
            "tokyo-night" => Self::new("#1a1b26", "#c0caf5", "#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#565f89", "#7aa2f7"),
            "catppuccin" => Self::new("#1e1e2e", "#cdd6f4", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#6c7086", "#cba6f7"),
            "gruvbox" => Self::new("#282828", "#ebdbb2", "#fb4934", "#b8bb26", "#fabd2f", "#83a598", "#d3869b", "#8ec07c", "#928374", "#fe8019"),
            "nord" => Self::new("#2e3440", "#d8dee9", "#bf616a", "#a3be8c", "#ebcb8b", "#81a1c1", "#b48ead", "#88c0d0", "#616e88", "#88c0d0"),
            "everforest" => Self::new("#2d353b", "#d3c6aa", "#e67e80", "#a7c080", "#dbbc7f", "#7fbbb3", "#d699b6", "#83c092", "#7a8478", "#a7c080"),
            "rose-pine" => Self::new("#191724", "#e0def4", "#eb6f92", "#9ccfd8", "#f6c177", "#31748f", "#c4a7e7", "#ebbcba", "#6e6a86", "#c4a7e7"),
            "kanagawa" => Self::new("#1f1f28", "#dcd7ba", "#e46876", "#98bb6c", "#e6c384", "#7e9cd8", "#957fb8", "#7fb4ca", "#727169", "#7e9cd8"),
            "matte-black" => Self::new("#121212", "#bebebe", "#d35f5f", "#8fa876", "#ffc107", "#e68e0d", "#b0a0c8", "#8a8a8d", "#5c5c5f", "#f59e0b"),
            _ => return None,
        })
    }

    /// Omarchy keeps the active theme under ~/.config/omarchy/current/theme.
    /// Newer releases ship a flat colors.toml; older ones only alacritty.toml.
    fn omarchy() -> Option<(String, Self)> {
        let dir = omarchy_theme_dir()?;
        let name = std::fs::canonicalize(&dir).ok()?.file_name()?.to_string_lossy().to_string();
        let palette = std::fs::read_to_string(dir.join("colors.toml"))
            .ok()
            .and_then(|raw| Self::from_colors_toml(&raw))
            .or_else(|| std::fs::read_to_string(dir.join("alacritty.toml")).ok().and_then(|raw| Self::from_alacritty(&raw)))?;
        Some((name, palette))
    }

    fn from_colors_toml(raw: &str) -> Option<Self> {
        let t: toml::Table = raw.parse().ok()?;
        let get = |k: &str| t.get(k).and_then(|v| v.as_str()).and_then(Rgb::hex);
        let blue = get("color4")?;
        Some(Self {
            bg: get("background")?,
            fg: get("foreground")?,
            red: get("color1")?,
            green: get("color2")?,
            yellow: get("color3")?,
            blue,
            magenta: get("color5")?,
            cyan: get("color6")?,
            bright_black: get("color8")?,
            accent: get("accent").unwrap_or(blue),
        })
    }

    fn from_alacritty(raw: &str) -> Option<Self> {
        let t: toml::Table = raw.parse().ok()?;
        let colors = t.get("colors")?.as_table()?;
        let get = |section: &str, k: &str| colors.get(section)?.as_table()?.get(k)?.as_str().and_then(Rgb::hex);
        let blue = get("normal", "blue")?;
        Some(Self {
            bg: get("primary", "background")?,
            fg: get("primary", "foreground")?,
            red: get("normal", "red")?,
            green: get("normal", "green")?,
            yellow: get("normal", "yellow")?,
            blue,
            magenta: get("normal", "magenta")?,
            cyan: get("normal", "cyan")?,
            bright_black: get("bright", "black").unwrap_or(blue),
            accent: blue,
        })
    }
}

/// 24-bit color unless we know the terminal can't do it. Apple's Terminal
/// doesn't advertise COLORTERM; ZABTERM_TRUECOLOR=1/0 forces either way.
fn truecolor() -> bool {
    static CACHE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CACHE.get_or_init(|| {
        if let Ok(v) = std::env::var("ZABTERM_TRUECOLOR") {
            return v != "0";
        }
        let colorterm = std::env::var("COLORTERM").unwrap_or_default();
        if colorterm.contains("truecolor") || colorterm.contains("24bit") {
            return true;
        }
        std::env::var("TERM_PROGRAM").map(|p| p != "Apple_Terminal").unwrap_or(true)
    })
}

fn omarchy_theme_dir() -> Option<PathBuf> {
    let dir = home().join(".config/omarchy/current/theme");
    dir.exists().then_some(dir)
}

/// Cheap fingerprint of the active Omarchy theme, polled to live-reload.
pub fn omarchy_signature() -> Option<(PathBuf, Option<SystemTime>)> {
    let dir = omarchy_theme_dir()?;
    let real = std::fs::canonicalize(&dir).ok()?;
    let mtime = ["colors.toml", "alacritty.toml"].iter().find_map(|f| std::fs::metadata(real.join(f)).and_then(|m| m.modified()).ok());
    Some((real, mtime))
}

impl Theme {
    pub fn load(name: &str) -> Self {
        if name == "auto" || name == "omarchy" {
            if let Some((theme_name, palette)) = Palette::omarchy() {
                return Self::from_palette(format!("omarchy:{theme_name}"), &palette);
            }
            return Self::load("tokyo-night");
        }
        match Palette::builtin(name) {
            Some(p) => Self::from_palette(name.to_string(), &p),
            None => Self::load("tokyo-night"),
        }
    }

    fn from_palette(name: String, p: &Palette) -> Self {
        let muted = p.fg.mix(p.bg, 0.45);
        Self {
            name,
            bg: p.bg.color(),
            fg: p.fg.color(),
            muted: muted.color(),
            border: p.bright_black.mix(p.bg, 0.25).color(),
            surface: p.bg.mix(p.accent, 0.16).color(),
            accent: p.accent.color(),
            ok: p.green.color(),
            warn: p.yellow.color(),
            err: p.red.color(),
            info: p.blue.color(),
            sev: [
                p.bright_black.mix(p.fg, 0.35).color(),
                p.blue.color(),
                p.yellow.color(),
                p.yellow.mix(p.red, 0.4).color(),
                p.yellow.mix(p.red, 0.75).color(),
                p.red.color(),
            ],
            series: [p.accent.color(), p.green.color(), p.magenta.color(), p.cyan.color(), p.yellow.color(), p.red.color()],
        }
    }

    pub fn next_name(current: &str) -> String {
        let mut names: Vec<&str> = Vec::new();
        if omarchy_theme_dir().is_some() {
            names.push("auto");
        }
        names.extend(BUILTIN);
        let key = if current.starts_with("omarchy:") { "auto" } else { current };
        let idx = names.iter().position(|n| *n == key).map(|i| (i + 1) % names.len()).unwrap_or(0);
        names[idx].to_string()
    }

    /// Terminal black, useful as text color on top of badges.
    pub fn on_badge(&self) -> Color {
        self.bg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_to_256_colors() {
        assert_eq!(Rgb(0, 0, 0).ansi256(), 16);
        assert_eq!(Rgb(255, 0, 0).ansi256(), 196);
        assert_eq!(Rgb(128, 128, 128).ansi256(), 244);
    }

    #[test]
    fn parses_alacritty_theme() {
        let raw = r##"
[colors.primary]
background = '#1a1b26'
foreground = '#a9b1d6'
[colors.normal]
black = '#32344a'
red = '#f7768e'
green = '#9ece6a'
yellow = '#e0af68'
blue = '#7aa2f7'
magenta = '#ad8ee6'
cyan = '#449dab'
white = '#787c99'
[colors.bright]
black = '#444b6a'
"##;
        let p = Palette::from_alacritty(raw).expect("parses");
        assert_eq!(p.red, Rgb(0xf7, 0x76, 0x8e));
        assert_eq!(p.bright_black, Rgb(0x44, 0x4b, 0x6a));
    }

    #[test]
    fn parses_colors_toml() {
        let mut raw = String::from("accent = \"#7aa2f7\"\nbackground = \"#1a1b26\"\nforeground = \"#a9b1d6\"\n");
        for i in 0..16 {
            raw.push_str(&format!("color{i} = \"#{:02x}{:02x}{:02x}\"\n", i * 10, i * 10, i * 10));
        }
        let p = Palette::from_colors_toml(&raw).expect("parses");
        assert_eq!(p.accent, Rgb(0x7a, 0xa2, 0xf7));
        assert_eq!(p.bright_black, Rgb(80, 80, 80));
    }
}
