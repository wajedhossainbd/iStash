//! Network details for the Overview: the active connection (read locally), and an on-demand speed test.
//! The speed test is the only thing here that goes online, and only when the user asks: it measures against
//! Cloudflare's public speed-test servers and looks up the public address and provider at ipinfo.io.

use serde_json::{json, Value};
use std::process::Command;

fn run(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd).args(args).output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
}

fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|l| l.trim().strip_prefix(key).map(str::trim))
}

/// The connection traffic goes out through: its kind ("Wi-Fi", "Ethernet"), local address and router.
#[tauri::command(async)]
pub fn net_info() -> Value {
    let route = run("/sbin/route", &["-n", "get", "default"]);
    let Some(iface) = field(&route, "interface:").map(str::to_string) else { return json!(null) };
    let gateway = field(&route, "gateway:").unwrap_or_default().to_string();
    let ip = run("/usr/sbin/ipconfig", &["getifaddr", &iface]).trim().to_string();
    let ports = run("/usr/sbin/networksetup", &["-listallhardwareports"]);
    let mut kind = String::new();
    let mut last = "";
    for l in ports.lines() {
        if let Some(p) = l.strip_prefix("Hardware Port: ") {
            last = p;
        } else if l.strip_prefix("Device: ") == Some(iface.as_str()) {
            kind = last.to_string();
        }
    }
    json!({ "iface": iface, "kind": if kind.is_empty() { iface.clone() } else { kind }, "ip": ip, "gateway": gateway })
}

fn curl(args: &[&str]) -> String {
    run("/usr/bin/curl", &[&["-s", "-o", "/dev/null", "-m", "20"][..], args].concat())
}

/// Download, upload (Mbit/s), ping and jitter (ms), plus public address, provider and place.
#[tauri::command(async)]
pub fn speed_test(server: Option<String>) -> Result<Value, String> {
    const CF: &str = "https://speed.cloudflare.com";
    // A custom server is any large file the user points at: download speed and ping only.
    let custom = server.filter(|s| !s.trim().is_empty());
    if let Some(u) = &custom {
        if !(u.starts_with("https://") || u.starts_with("http://")) || u.contains(char::is_whitespace) {
            return Err("Enter a full web address starting with https://.".into());
        }
    }
    let probe = custom.clone().unwrap_or_else(|| format!("{CF}/__down?bytes=0"));
    // Ping: time from request sent to first byte, for an empty response; the median of six.
    let mut rtt: Vec<f64> = (0..6)
        .filter_map(|_| {
            let t = curl(&["-r", "0-0", "-w", "%{time_pretransfer} %{time_starttransfer}", &probe]);
            let v: Vec<f64> = t.split_whitespace().filter_map(|x| x.parse().ok()).collect();
            (v.len() == 2).then(|| (v[1] - v[0]) * 1000.0)
        })
        .collect();
    if rtt.is_empty() {
        return Err("The speed test couldn't reach the internet. Check your connection and try again.".into());
    }
    let jitter = rtt.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>() / (rtt.len().max(2) - 1) as f64;
    rtt.sort_by(f64::total_cmp);
    let ping = rtt[rtt.len() / 2];
    let speed = |s: String| s.trim().parse::<f64>().unwrap_or(0.0) * 8.0 / 1e6;
    let down = match &custom {
        Some(u) => speed(run("/usr/bin/curl", &["-s", "-o", "/dev/null", "-m", "15", "-L", "-w", "%{speed_download}", u])),
        None => speed(curl(&["-w", "%{speed_download}", &format!("{CF}/__down?bytes=25000000")])),
    };
    let up = custom.is_none().then(|| speed(run("/bin/sh", &["-c", &format!("head -c 10000000 /dev/zero | /usr/bin/curl -s -o /dev/null -m 20 -w '%{{speed_upload}}' --data-binary @- {CF}/__up")])));
    let server_name = custom.as_deref().and_then(|u| u.split('/').nth(2)).unwrap_or("Cloudflare").to_string();
    let who: Value = serde_json::from_str(&run("/usr/bin/curl", &["-s", "-m", "8", "https://ipinfo.io/json"])).unwrap_or(Value::Null);
    let s = |k: &str| who[k].as_str().unwrap_or("").to_string();
    // ipinfo gives "AS24389 Grameenphone Ltd."; the AS number means nothing to people.
    let isp = s("org").split_once(' ').map_or(s("org"), |(_, rest)| rest.to_string());
    Ok(json!({ "down": down, "up": up, "ping": ping, "jitter": jitter, "ip": s("ip"), "isp": isp, "city": s("city"), "country": s("country"), "server": server_name, "at": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64) }))
}
