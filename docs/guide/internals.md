---
title: "How it works"
lead: "What zabterm asks your Zabbix for, how often, and which item keys it reads. Useful if you run a big installation or restrict API permissions."
---

## Architecture

zabterm has two halves connected by channels:

- A **background task** owns the API client. It polls every `refresh_interval` seconds and on demand (refresh, opening a host, acknowledging), and sends finished snapshots to the UI.
- The **UI loop** redraws four times a second from the latest snapshot and handles keys. It never waits on the network, so the interface stays responsive even when the API is slow or down.

## API calls per poll

| Call | Purpose |
|---|---|
| `host.get` | Hosts with interfaces, host groups, status and maintenance |
| `problem.get` | Up to 500 active problems with tags |
| `trigger.get` | Maps problems to their hosts, reads `manual_close` |
| `item.get` (exact keys) | CPU, memory, load, uptime, CPU count |
| `item.get` (key prefixes) | Network and filesystem items |

The first four run in parallel. The first poll also calls `history.get` once to fill the 30 minute sparklines; after that they grow from each poll's last values, so steady-state polling never re-reads history.

Opening a host adds `item.get` for all of its items and `history.get` for the chosen range, refreshed on every poll while the page is open. Acknowledging calls `event.acknowledge`.

## Item keys

The overview columns come from the keys used by the standard *Linux by Zabbix agent* template:

| Column | Key |
|---|---|
| CPU | `system.cpu.util` |
| Memory | `vm.memory.util` |
| Load | `system.cpu.load[all,avg1]` |
| Uptime | `system.uptime` |
| CPUs | `system.cpu.num` |
| Network | `net.if.in["IFACE"]`, `net.if.out["IFACE"]` summed over interfaces, loopback excluded |
| Disk | `vfs.fs.dependent.size[/,pused]`, or the fullest mount if `/` is not monitored |

Hosts without these items still appear, with `-` in the empty columns. The host page and its Latest data tab show every item regardless of key.

## Availability

A host counts as **up** when its agent interface (or its first interface) is available, or when active checks are available. It counts as **down** when either reports unavailable, and **unknown** otherwise. Unreachable hosts keep their last values on screen, dimmed, and are left out of averages.

## Permissions

The token's user needs read access to the host groups you want to see. Acknowledging and closing problems need those actions allowed in the user's role. zabterm does not change configuration.

## Talking to the API

- Requests are JSON-RPC over HTTPS with a `Bearer` token header and a 20 second timeout.
- TLS uses rustls with your system's certificate store. `verify_tls = false` turns verification off for that profile.
- Errors are shown as toasts and on the splash screen; the next poll retries automatically. With username and password, an expired session is replaced by a fresh login.

## Scale

Each poll is a handful of calls no matter how many hosts you have, but the size of each response grows with the fleet. On large installations, raise `refresh_interval` to 30 or 60 seconds and use a token whose user only sees the host groups you care about. The network and disk columns come from a key-prefix search that is the slowest call on big instances; it gets a 5 second budget, and if it fails those columns stay empty and it is retried a minute later, so the rest of the dashboard is not held up.
