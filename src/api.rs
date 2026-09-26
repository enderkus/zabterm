//! Minimal async Zabbix JSON-RPC client plus the handful of models zabterm needs.
//!
//! Zabbix returns almost every number as a string, so models keep the raw
//! strings from the wire and parse them where they are used.

use crate::config::{Auth, Profile};
use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

pub struct Client {
    http: reqwest::Client,
    url: String,
    token: Option<String>,
    id: AtomicU64,
}

impl Client {
    pub async fn connect(profile: &Profile) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .danger_accept_invalid_certs(!profile.verify_tls)
            .user_agent(concat!("zabterm/", env!("CARGO_PKG_VERSION")))
            .build()?;
        let mut client = Self { http, url: profile.api_url(), token: None, id: AtomicU64::new(1) };
        client.token = Some(match profile.resolve_auth()? {
            Auth::Token(token) => token,
            Auth::Login { username, password } => {
                client.call("user.login", json!({ "username": username, "password": password })).await.context("user.login")?
            }
        });
        Ok(client)
    }

    pub async fn call<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T> {
        let body = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": self.id.fetch_add(1, Ordering::Relaxed),
        });
        let mut req = self.http.post(&self.url).header("Content-Type", "application/json-rpc").json(&body);
        // apiinfo.version and user.login refuse to run with credentials attached.
        if let Some(token) = self.token.as_ref().filter(|_| method != "apiinfo.version" && method != "user.login") {
            req = req.bearer_auth(token);
        }
        let resp = req.send().await.with_context(|| format!("POST {}", self.url))?;
        let status = resp.status();
        if !status.is_success() {
            bail!("{method}: HTTP {status}");
        }
        let mut value: Value = resp.json().await.with_context(|| format!("{method}: invalid JSON"))?;
        if let Some(err) = value.get("error") {
            let msg = err.get("message").and_then(Value::as_str).unwrap_or("error");
            let data = err.get("data").and_then(Value::as_str).unwrap_or("");
            return Err(anyhow!("{method}: {msg} {data}"));
        }
        let result = value.get_mut("result").map(Value::take).ok_or_else(|| anyhow!("{method}: no result"))?;
        serde_json::from_value(result).with_context(|| format!("{method}: unexpected response shape"))
    }

    pub async fn version(&self) -> Result<String> {
        self.call("apiinfo.version", json!({})).await
    }

    pub async fn hosts(&self) -> Result<Vec<Host>> {
        self.call(
            "host.get",
            json!({
                "output": ["hostid", "name", "status", "maintenance_status", "active_available"],
                "selectInterfaces": ["ip", "dns", "useip", "port", "available", "type", "error"],
                "selectHostGroups": ["name"],
                "sortfield": "name",
            }),
        )
        .await
    }

    pub async fn problems(&self) -> Result<Vec<RawProblem>> {
        self.call(
            "problem.get",
            json!({
                "output": ["eventid", "objectid", "name", "severity", "clock", "acknowledged", "suppressed", "opdata"],
                "selectTags": ["tag", "value"],
                "sortfield": ["eventid"],
                "sortorder": "DESC",
                "limit": 500,
            }),
        )
        .await
    }

    pub async fn trigger_hosts(&self, triggerids: &[String]) -> Result<Vec<Trigger>> {
        if triggerids.is_empty() {
            return Ok(vec![]);
        }
        self.call(
            "trigger.get",
            json!({
                "triggerids": triggerids,
                "output": ["triggerid", "description", "manual_close"],
                "selectHosts": ["hostid", "name"],
            }),
        )
        .await
    }

    /// Items whose key matches one of `keys` exactly.
    pub async fn items_by_key(&self, hostids: Option<&[String]>, keys: &[&str]) -> Result<Vec<Item>> {
        let mut params = json!({
            "output": ["itemid", "hostid", "name", "key_", "lastvalue", "lastclock", "units", "value_type"],
            "filter": { "key_": keys },
            "monitored": true,
        });
        if let Some(ids) = hostids {
            params["hostids"] = json!(ids);
        }
        self.call("item.get", params).await
    }

    /// Items whose key starts with any of `prefixes`.
    pub async fn items_by_prefix(&self, hostids: Option<&[String]>, prefixes: &[&str]) -> Result<Vec<Item>> {
        let mut params = json!({
            "output": ["itemid", "hostid", "name", "key_", "lastvalue", "lastclock", "units", "value_type"],
            "search": { "key_": prefixes },
            "searchByAny": true,
            "startSearch": true,
            "monitored": true,
        });
        if let Some(ids) = hostids {
            params["hostids"] = json!(ids);
        }
        self.call("item.get", params).await
    }

    pub async fn host_items(&self, hostid: &str) -> Result<Vec<Item>> {
        self.call(
            "item.get",
            json!({
                "hostids": [hostid],
                "output": ["itemid", "hostid", "name", "key_", "lastvalue", "lastclock", "units", "value_type"],
                "sortfield": "name",
            }),
        )
        .await
    }

    /// history.get only accepts one value type per call.
    pub async fn history(&self, itemids: &[String], value_type: u8, from: i64) -> Result<Vec<HistoryPoint>> {
        if itemids.is_empty() {
            return Ok(vec![]);
        }
        self.call(
            "history.get",
            json!({
                "itemids": itemids,
                "history": value_type,
                "time_from": from,
                "sortfield": "clock",
                "sortorder": "ASC",
            }),
        )
        .await
    }

    pub async fn acknowledge(&self, eventids: &[String], message: &str, close: bool) -> Result<()> {
        // action bitmask: 1 close, 2 acknowledge, 4 add message
        let mut action = 2;
        if !message.is_empty() {
            action |= 4;
        }
        if close {
            action |= 1;
        }
        let _: Value = self.call("event.acknowledge", json!({ "eventids": eventids, "action": action, "message": message })).await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Host {
    pub hostid: String,
    pub name: String,
    pub status: String,
    pub maintenance_status: String,
    #[serde(default)]
    pub active_available: String,
    #[serde(default)]
    pub interfaces: Vec<Interface>,
    #[serde(default)]
    pub hostgroups: Vec<Named>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Interface {
    pub ip: String,
    pub dns: String,
    pub useip: String,
    pub port: String,
    pub available: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub error: String,
}

impl Interface {
    pub fn address(&self) -> String {
        let addr = if self.useip == "1" { &self.ip } else { &self.dns };
        format!("{addr}:{}", self.port)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Named {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawProblem {
    pub eventid: String,
    pub objectid: String,
    pub name: String,
    pub severity: String,
    pub clock: String,
    pub acknowledged: String,
    pub suppressed: String,
    #[serde(default)]
    pub opdata: String,
    #[serde(default)]
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Tag {
    pub tag: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Trigger {
    pub triggerid: String,
    #[serde(default)]
    pub manual_close: String,
    #[serde(default)]
    pub hosts: Vec<TriggerHost>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TriggerHost {
    pub hostid: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    pub itemid: String,
    pub hostid: String,
    pub name: String,
    pub key_: String,
    pub lastvalue: String,
    pub lastclock: String,
    pub units: String,
    pub value_type: String,
}

impl Item {
    pub fn value(&self) -> Option<f64> {
        if self.lastclock == "0" { None } else { self.lastvalue.parse().ok() }
    }

    pub fn is_numeric(&self) -> bool {
        self.value_type == "0" || self.value_type == "3"
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct HistoryPoint {
    pub itemid: String,
    pub clock: String,
    pub value: String,
}
