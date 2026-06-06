# rust-monitor 🦀

A lightweight system monitor that posts live server statistics to a Discord webhook. Built in Rust for maximum efficiency — minimal CPU and memory footprint, zero runtime dependencies.

## Features

- **Live monitoring** — collects CPU load, RAM usage, disk usage, and uptime
- **Dual server support** — monitors a local server plus a remote server via SSH
- **Discord integration** — posts updates to a Discord webhook as a compact embed
- **Efficient** — tiny memory footprint (~1 MB RSS), instant startup, no Python interpreter overhead
- **Environment-driven config** — all settings via environment variables, no hardcoded secrets

## How it works

The binary runs as a long-lived process. Every 30 seconds (configurable) it:

1. Reads local system stats (`/proc/loadavg`, `sysinfo` crate, disk usage)
2. SSHs into a remote server to collect its stats
3. Sends a PATCH request to a Discord webhook, updating the same embed message

Both servers are displayed side-by-side in a single Discord embed with inline fields.

## Requirements

- Linux (uses `/proc/loadavg` and `sysinfo` APIs)
- SSH key access to a remote server (optional — omit `MONITOR_REMOTE_HOST` to skip)

## Installation

### 1. Download or build

```bash
git clone https://github.com/ZoniBoy00/rust-monitor.git
cd rust-monitor
cargo build --release
sudo cp target/release/rust-monitor /usr/local/bin/
```

Or download a pre-built binary from the [releases page](https://github.com/ZoniBoy00/rust-monitor/releases).

### 2. Configure

```bash
# Create a Discord webhook (Discord channel settings → Integrations → Webhook)
# Send any message to the channel, copy its ID for MONITOR_MESSAGE_ID

export MONITOR_WEBHOOK_URL=https://discord.com/api/webhooks/your_id/your_token
export MONITOR_MESSAGE_ID=your_message_id
export MONITOR_REMOTE_HOST=your_remote_server_ip
export MONITOR_SSH_KEY_PATH=/home/user/.ssh/id_ed25519
```

Or copy `.env.example` to a file and use an env-file approach with your service manager.

### 3. Run

```bash
rust-monitor
```

### 4. systemd service (recommended)

Create `/etc/systemd/system/rust-monitor.service`:

```ini
[Unit]
Description=Rust System Monitor - Discord status webhook
After=network.target

[Service]
Type=simple
User=root
EnvironmentFile=/etc/rust-monitor.env
ExecStart=/usr/local/bin/rust-monitor
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Then:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now rust-monitor
```

## Environment variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `MONITOR_WEBHOOK_URL` | ✅ Yes | — | Discord webhook URL |
| `MONITOR_MESSAGE_ID` | ✅ Yes | — | Discord message ID to update |
| `MONITOR_REMOTE_HOST` | ❌ No | `127.0.0.1` | Remote server IP/hostname |
| `MONITOR_REMOTE_USER` | ❌ No | `root` | Remote SSH user |
| `MONITOR_SSH_KEY_PATH` | ❌ No | `~/.ssh/id_ed25519` | SSH private key path |
| `MONITOR_INTERVAL_SECS` | ❌ No | `30` | Update interval in seconds |
| `MONITOR_LOCAL_NAME` | ❌ No | `🏠 Local Server` | Display name in embed |
| `MONITOR_REMOTE_NAME` | ❌ No | `☁️ Remote Server` | Display name in embed |

## Discord embed preview

The embed shows two inline fields side by side:

- **Local Server** — status, uptime, load, RAM (used/total), disk usage %
- **Remote Server** — same metrics, collected via SSH

Status shows 🟢 Online or 🔴 Offline based on SSH reachability.

## License

MIT
