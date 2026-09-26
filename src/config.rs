//! Configuration: `$XDG_CONFIG_HOME/zabterm/config.toml` (falls back to `~/.config`).
//!
//! A config holds global UI settings plus one or more named profiles, so a
//! single binary can hop between prod, staging and a local lab.

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

pub const TEMPLATE: &str = r#"# zabterm: a terminal UI for Zabbix
#
# Everything here is optional except a profile with a `url` and some way to
# authenticate. Environment variables ZABTERM_URL and ZABTERM_TOKEN override the
# active profile, handy for one-off sessions.

# Profile used when --profile is not given.
default_profile = "local"

# "auto" follows your current Omarchy theme (and live-reloads when you switch
# it), falling back to tokyo-night everywhere else. Built-ins: tokyo-night,
# catppuccin, gruvbox, nord, everforest, rose-pine, kanagawa, matte-black.
theme = "auto"

# Seconds between API polls.
refresh_interval = 10

[notifications]
# Fire notify-send (mako, dunst, ...) when a new problem shows up.
desktop = true
# Ignore anything below this: not_classified, information, warning,
# average, high, disaster.
min_severity = "average"

[profiles.local]
url = "http://localhost:8080"
# Create one in Zabbix: User settings -> API tokens.
token = "paste-your-api-token-here"

# Prefer not to keep secrets in plain text? Pick one of these instead:
# token_env = "ZABBIX_TOKEN"
# token_cmd = "pass show zabbix/local"      # or: op read op://vault/zabbix/token
#
# Or classic username/password (a session is opened with user.login):
# username = "Admin"
# password = "zabbix"

# Self-signed certificate on the frontend?
# verify_tls = false

# [profiles.prod]
# url = "https://zabbix.example.com"
# token_cmd = "op read op://ops/zabbix-prod/token"
"#;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub default_profile: Option<String>,
    pub theme: String,
    pub refresh_interval: u64,
    pub notifications: Notifications,
    pub profiles: BTreeMap<String, Profile>,
}

impl Default for Config {
    fn default() -> Self {
        Self { default_profile: None, theme: "auto".into(), refresh_interval: 10, notifications: Notifications::default(), profiles: BTreeMap::new() }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Notifications {
    pub desktop: bool,
    pub min_severity: String,
}

impl Default for Notifications {
    fn default() -> Self {
        Self { desktop: true, min_severity: "average".into() }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Profile {
    #[serde(skip)]
    pub name: String,
    pub url: String,
    pub token: Option<String>,
    pub token_env: Option<String>,
    pub token_cmd: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    #[serde(default = "yes")]
    pub verify_tls: bool,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone)]
pub enum Auth {
    Token(String),
    Login { username: String, password: String },
}

impl Profile {
    /// Full JSON-RPC endpoint, whether the user wrote the base URL or the
    /// whole `api_jsonrpc.php` path.
    pub fn api_url(&self) -> String {
        let url = self.url.trim_end_matches('/');
        if url.ends_with("api_jsonrpc.php") { url.to_string() } else { format!("{url}/api_jsonrpc.php") }
    }

    pub fn web_url(&self) -> String {
        self.url.trim_end_matches('/').trim_end_matches("api_jsonrpc.php").trim_end_matches('/').to_string()
    }

    pub fn resolve_auth(&self) -> Result<Auth> {
        if let Ok(token) = std::env::var("ZABTERM_TOKEN") {
            return Ok(Auth::Token(token));
        }
        if let Some(var) = &self.token_env {
            let token = std::env::var(var).with_context(|| format!("token_env: ${var} is not set"))?;
            return Ok(Auth::Token(token));
        }
        if let Some(cmd) = &self.token_cmd {
            let out = Command::new("sh").arg("-c").arg(cmd).output().context("token_cmd failed to start")?;
            if !out.status.success() {
                bail!("token_cmd exited with {}: {}", out.status, String::from_utf8_lossy(&out.stderr).trim());
            }
            return Ok(Auth::Token(String::from_utf8_lossy(&out.stdout).trim().to_string()));
        }
        if let Some(token) = &self.token
            && !token.is_empty()
            && token != "paste-your-api-token-here"
        {
            return Ok(Auth::Token(token.clone()));
        }
        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            return Ok(Auth::Login { username: username.clone(), password: password.clone() });
        }
        bail!("profile '{}' has no credentials: set token, token_env, token_cmd or username/password", self.name)
    }
}

pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| home().join(".config")).join("zabterm")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

impl Config {
    pub fn load(path: Option<&PathBuf>) -> Result<Self> {
        let path = path.cloned().unwrap_or_else(config_path);
        let mut config = if path.exists() {
            let raw = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            toml::from_str::<Config>(&raw).with_context(|| format!("parsing {}", path.display()))?
        } else {
            Config::default()
        };
        for (name, profile) in config.profiles.iter_mut() {
            profile.name = name.clone();
        }
        // ZABTERM_URL alone is enough to run without any config file at all.
        if let Ok(url) = std::env::var("ZABTERM_URL") {
            config.profiles.insert("env".into(), Profile { name: "env".into(), url, verify_tls: true, ..Default::default() });
            config.default_profile = Some("env".into());
        }
        if config.profiles.is_empty() {
            bail!(
                "no profiles configured.\n\n  Run `zabterm init` to write a starter config to {}\n  or export ZABTERM_URL and ZABTERM_TOKEN.",
                path.display()
            );
        }
        Ok(config)
    }

    pub fn profile(&self, name: Option<&str>) -> Result<Profile> {
        let wanted = name.map(str::to_string).or_else(|| self.default_profile.clone());
        match wanted {
            Some(n) => self
                .profiles
                .get(&n)
                .cloned()
                .ok_or_else(|| anyhow!("profile '{n}' not found (have: {})", self.profiles.keys().cloned().collect::<Vec<_>>().join(", "))),
            None => Ok(self.profiles.values().next().cloned().expect("profiles is non-empty")),
        }
    }
}

pub fn write_template(force: bool) -> Result<PathBuf> {
    let path = config_path();
    if path.exists() && !force {
        bail!("{} already exists (use --force to overwrite)", path.display());
    }
    std::fs::create_dir_all(config_dir())?;
    std::fs::write(&path, TEMPLATE)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(path)
}
