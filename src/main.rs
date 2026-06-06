use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use std::env;
use std::fmt::Write as _;
use std::path::Path;
use std::time::Duration;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, System};

#[derive(Debug, Default)]
struct Stats {
    uptime: String,
    load_1: String,
    mem_used_mb: u64,
    mem_total_mb: u64,
    disk_use_pct: String,
}

#[derive(Debug)]
struct Config {
    webhook_url: String,
    message_id: String,
    ssh_key_path: String,
    remote_host: String,
    remote_user: String,
    interval_secs: u64,
}

fn load_config() -> Config {
    Config {
        webhook_url: env::var("MONITOR_WEBHOOK_URL").expect("MONITOR_WEBHOOK_URL must be set"),
        message_id: env::var("MONITOR_MESSAGE_ID").expect("MONITOR_MESSAGE_ID must be set"),
        ssh_key_path: env::var("MONITOR_SSH_KEY_PATH")
            .unwrap_or_else(|_| {
                let home = env::var("HOME").unwrap_or_default();
                format!("{}/.ssh/id_ed25519", home)
            }),
        remote_host: env::var("MONITOR_REMOTE_HOST")
            .unwrap_or_else(|_| "127.0.0.1".to_string()),
        remote_user: env::var("MONITOR_REMOTE_USER")
            .unwrap_or_else(|_| "root".to_string()),
        interval_secs: env::var("MONITOR_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30),
    }
}

fn collect_stats(sys: &mut System) -> Stats {
    sys.refresh_cpu_specifics(CpuRefreshKind::nothing());
    sys.refresh_memory_specifics(MemoryRefreshKind::everything());

    let uptime_secs = System::uptime();
    let uptime = format_uptime(uptime_secs);

    let load_1 = std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|s| s.split_whitespace().next().map(|s| s.to_string()))
        .unwrap_or_default();

    let mem_total_mb = sys.total_memory() / 1024 / 1024;
    let mem_used_mb = sys.used_memory() / 1024 / 1024;

    let disk_use_pct = Disks::new_with_refreshed_list()
        .iter()
        .find(|d| d.mount_point() == Path::new("/"))
        .map(|d| {
            let total = d.total_space();
            let available = d.available_space();
            if total > 0 {
                let used_pct = ((total - available) as f64 / total as f64 * 100.0) as u64;
                format!("{used_pct}%")
            } else {
                "N/A".to_string()
            }
        })
        .unwrap_or_else(|| "N/A".to_string());

    Stats {
        uptime,
        load_1,
        mem_used_mb,
        mem_total_mb,
        disk_use_pct,
    }
}

fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;

    let mut out = String::new();
    if days > 0 {
        let _ = write!(out, "{days} days, ");
    }
    if hours > 0 || days > 0 {
        let _ = write!(out, "{hours} hours, ");
    }
    let _ = write!(out, "{minutes} minutes");
    out
}

fn fetch_remote_stats(config: &Config) -> (String, String, String, String, String) {
    let result = std::process::Command::new("ssh")
        .args([
            "-i",
            &config.ssh_key_path,
            "-o",
            "ConnectTimeout=3",
            "-o",
            "StrictHostKeyChecking=no",
            &format!("{}@{}", config.remote_user, config.remote_host),
            "uptime -p && cat /proc/loadavg && free -m | grep Mem && df -h / | tail -1",
        ])
        .output();

    match result {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = stdout.lines().collect();

            let r_up = lines
                .first()
                .map(|s| s.trim().strip_prefix("up ").unwrap_or(s.trim()))
                .unwrap_or("N/A")
                .to_string();
            let r_load = lines
                .get(1)
                .and_then(|s| s.split_whitespace().next())
                .unwrap_or("N/A")
                .to_string();
            let (r_mem_used, r_mem_total) = lines
                .get(2)
                .and_then(|s| {
                    let parts: Vec<&str> = s.split_whitespace().collect();
                    if parts.len() >= 3 {
                        Some((parts[2].to_string(), parts[1].to_string()))
                    } else {
                        None
                    }
                })
                .unwrap_or(("N/A".into(), "N/A".into()));
            let r_disk = lines
                .get(3)
                .and_then(|s| s.split_whitespace().nth(4))
                .unwrap_or("N/A")
                .to_string();
            let r_mem = format!("{r_mem_used}/{r_mem_total} MB");

            (r_up, r_load, r_mem, r_disk, "🟢 Online".into())
        }
        _ => (
            "N/A".into(),
            "N/A".into(),
            "N/A".into(),
            "N/A".into(),
            "🔴 Offline".into(),
        ),
    }
}

fn build_embed(
    local: &Stats,
    r_up: &str,
    r_load: &str,
    r_mem: &str,
    r_disk: &str,
    remote_status: &str,
    local_name: &str,
    remote_name: &str,
) -> serde_json::Value {
    let color = if remote_status == "🟢 Online" {
        3066993
    } else {
        15158332
    };

    json!({
        "embeds": [{
            "title": "📡 Live System Monitor",
            "description": format!("Last updated: <t:{}:R>", Utc::now().timestamp()),
            "color": color,
            "fields": [
                {
                    "name": local_name,
                    "value": format!(
                        "**Status:** 🟢 Online\n**Uptime:** {}\n**Load:** {}\n**RAM:** {}/{} MB\n**Disk:** {}",
                        local.uptime, local.load_1, local.mem_used_mb, local.mem_total_mb, local.disk_use_pct
                    ),
                    "inline": true,
                },
                {
                    "name": remote_name,
                    "value": format!(
                        "**Status:** {remote_status}\n**Uptime:** {r_up}\n**Load:** {r_load}\n**RAM:** {r_mem}\n**Disk:** {r_disk}"
                    ),
                    "inline": true,
                },
            ],
            "footer": { "text": "Live data updates every 30s • rust-monitor" },
            "timestamp": Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        }]
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config();

    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let mut sys = System::new();
    let url = format!("{}/messages/{}", config.webhook_url, config.message_id);
    let mut tick: u64 = 0;
    let mut first = true;

    let local_name = env::var("MONITOR_LOCAL_NAME").unwrap_or_else(|_| "🏠 Local Server".to_string());
    let remote_name = env::var("MONITOR_REMOTE_NAME").unwrap_or_else(|_| "☁️ Remote Server".to_string());

    eprintln!("[rust-monitor] Starting, msg_id={}", config.message_id);

    loop {
        let local = collect_stats(&mut sys);
        let (r_up, r_load, r_mem, r_disk, r_status) = fetch_remote_stats(&config);
        let embed = build_embed(&local, &r_up, &r_load, &r_mem, &r_disk, &r_status, &local_name, &remote_name);

        match client.patch(&url).json(&embed).send().await {
            Ok(r) if r.status().is_success() => {
                tick += 1;
                if first {
                    eprintln!("[rust-monitor] First update sent!");
                    first = false;
                } else if tick % 10 == 0 {
                    eprintln!("[rust-monitor] ❤️ OK — {tick} updates sent");
                }
            }
            Ok(r) => eprintln!("[rust-monitor] HTTP {}", r.status()),
            Err(e) => eprintln!("[rust-monitor] Error: {e}"),
        }

        tokio::time::sleep(Duration::from_secs(config.interval_secs)).await;
    }
}
