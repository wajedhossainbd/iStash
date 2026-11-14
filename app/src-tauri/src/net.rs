//! Network details for the Overview: the active connection (read locally), and an on-demand speed test.
//! The speed test is the only thing here that goes online, and only when the user asks: it measures against
//! Cloudflare, a Speedtest (Ookla) server from their public list, or a file the user picks, and looks up the public
//! address and provider at ipinfo.io.

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

/// Speed-test servers from the public Speedtest server list: the nearest ones, or those matching a search.
#[tauri::command(async)]
pub fn speed_servers(query: Option<String>) -> Value {
    let q = query.unwrap_or_default();
    let mut url = "https://www.speedtest.net/api/js/servers?engine=js&limit=20".to_string();
    if !q.trim().is_empty() {
        let enc: String = q.trim().chars().map(|c| if c.is_ascii_alphanumeric() { c.to_string() } else { format!("%{:02X}", c as u32 & 0xff) }).collect();
        url.push_str(&format!("&search={enc}"));
    }
    let list: Value = serde_json::from_str(&run("/usr/bin/curl", &["-s", "-m", "10", &url])).unwrap_or(Value::Null);
    let rows: Vec<Value> = list.as_array().into_iter().flatten().filter_map(|s| {
        let host = s["host"].as_str()?;
        // Hosts are names with a port, nothing else; anything odd is skipped rather than passed to curl.
        if !host.chars().all(|c| c.is_ascii_alphanumeric() || ".-:".contains(c)) { return None; }
        Some(json!({ "host": host, "sponsor": s["sponsor"], "city": s["name"], "cc": s["cc"], "km": s["distance"] }))
    }).collect();
    Value::Array(rows)
}

/// One measured transfer: bytes per second, or None if it failed.
fn transfer(args: &[&str]) -> Option<f64> {
    let out = run("/usr/bin/curl", &[&["-s", "-o", "/dev/null", "-m", "12", "-w", "%{http_code} %{speed_download} %{speed_upload}"][..], args].concat());
    let v: Vec<&str> = out.split_whitespace().collect();
    let ok = v.first().is_some_and(|c| c.starts_with('2'));
    let speed = v.get(1).and_then(|d| d.parse::<f64>().ok()).unwrap_or(0.0).max(v.get(2).and_then(|u| u.parse::<f64>().ok()).unwrap_or(0.0));
    (ok && speed > 0.0).then_some(speed)
}

fn upload(url: &str, bytes: usize) -> Option<f64> {
    let cmd = format!("head -c {bytes} /dev/zero | /usr/bin/curl -s -o /dev/null -m 12 -w '%{{http_code}} %{{speed_upload}}' --data-binary @- '{url}'");
    let out = run("/bin/sh", &["-c", &cmd]);
    let v: Vec<&str> = out.split_whitespace().collect();
    let speed = v.get(1).and_then(|u| u.parse::<f64>().ok()).unwrap_or(0.0);
    (v.first().is_some_and(|c| c.starts_with('2')) && speed > 0.0).then_some(speed)
}

/// Runs a test against `server` ("" = Cloudflare nearest, "ookla:<host>", or a file URL) in `mode`
/// ("full", "down", "up"). Sends `speed-progress` events {phase, mbps, ping, pct} as it goes so the gauge moves.
#[tauri::command(async)]
pub fn speed_test(app: tauri::AppHandle, server: Option<String>, mode: Option<String>) -> Result<Value, String> {
    use tauri::Emitter;
    let server = server.unwrap_or_default();
    let mode = mode.unwrap_or_else(|| "full".into());
    let ookla = server.strip_prefix("ookla:").map(str::to_string);
    if let Some(h) = &ookla {
        if !h.chars().all(|c| c.is_ascii_alphanumeric() || ".-:".contains(c)) { return Err("That server address isn't valid.".into()); }
    }
    let file = (!server.is_empty() && ookla.is_none()).then(|| server.clone());
    if let Some(u) = &file {
        if !(u.starts_with("https://") || u.starts_with("http://")) || u.contains(|c: char| c.is_whitespace() || c == '\'') {
            return Err("Enter a full web address starting with https://.".into());
        }
    }
    let emit = |phase: &str, mbps: f64, ping: Option<f64>, pct: f64| { let _ = app.emit("speed-progress", json!({ "phase": phase, "mbps": mbps, "ping": ping, "pct": pct })); };
    const CF: &str = "https://speed.cloudflare.com";
    let probe = match (&ookla, &file) { (Some(h), _) => format!("http://{h}/hello"), (_, Some(u)) => u.clone(), _ => format!("{CF}/__down?bytes=0") };

    // Ping: time from request sent to first byte, median of eight; jitter is the average change between them.
    let mut rtt = vec![];
    for i in 0..8 {
        let t = run("/usr/bin/curl", &["-s", "-o", "/dev/null", "-m", "5", "-r", "0-0", "-w", "%{time_pretransfer} %{time_starttransfer}", &probe]);
        let v: Vec<f64> = t.split_whitespace().filter_map(|x| x.parse().ok()).collect();
        if v.len() == 2 && v[1] > 0.0 { rtt.push((v[1] - v[0]) * 1000.0); emit("ping", 0.0, rtt.last().copied(), (i + 1) as f64 / 8.0); }
    }
    if rtt.is_empty() { return Err("The server didn't answer. Try another one, or check your connection.".into()); }
    let jitter = rtt.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>() / (rtt.len().max(2) - 1) as f64;
    let mut sorted = rtt.clone();
    sorted.sort_by(f64::total_cmp);
    let ping = sorted[sorted.len() / 2];

    // Download and upload: transfers of growing size for about eight seconds each, reporting every one.
    let phase = |name: &str, run_one: &dyn Fn(usize) -> Option<f64>| -> Option<f64> {
        let (start, mut size, mut speeds) = (std::time::Instant::now(), 1_000_000usize, vec![]);
        while start.elapsed().as_secs_f64() < 8.0 {
            let Some(b) = run_one(size) else { break };
            speeds.push(b * 8.0 / 1e6);
            emit(name, *speeds.last().unwrap(), None, (start.elapsed().as_secs_f64() / 8.0).min(1.0));
            if b * 1.5 > size as f64 { size = (size * 2).min(50_000_000); }
        }
        // The best half of the readings: the first, small transfers undercount a fast line.
        speeds.sort_by(|a, b| b.total_cmp(a));
        let top = &speeds[..speeds.len().div_ceil(2).max(speeds.len().min(1))];
        (!top.is_empty()).then(|| top.iter().sum::<f64>() / top.len() as f64)
    };
    let down = (mode != "up").then(|| phase("down", &|n| match (&ookla, &file) {
        (Some(h), _) => transfer(&[&format!("http://{h}/download?nocache={n}&size={n}")]),
        (_, Some(u)) => transfer(&["-L", "-r", &format!("0-{}", n - 1), u]),
        _ => transfer(&[&format!("{CF}/__down?bytes={n}")]),
    })).flatten();
    let up = (mode != "down" && file.is_none()).then(|| phase("up", &|n| match &ookla {
        Some(h) => upload(&format!("http://{h}/upload?nocache={n}"), n),
        None => upload(&format!("{CF}/__up"), n),
    })).flatten();
    emit("done", down.or(up).unwrap_or(0.0), Some(ping), 1.0);

    let who: Value = serde_json::from_str(&run("/usr/bin/curl", &["-s", "-m", "8", "https://ipinfo.io/json"])).unwrap_or(Value::Null);
    let s = |k: &str| who[k].as_str().unwrap_or("").to_string();
    let isp = s("org").split_once(' ').map_or(s("org"), |(_, rest)| rest.to_string());
    let server_name = match (&ookla, &file) { (Some(h), _) => h.split(':').next().unwrap_or(h).to_string(), (_, Some(u)) => u.split('/').nth(2).unwrap_or("").to_string(), _ => "Cloudflare".into() };
    Ok(json!({ "down": down, "up": up, "ping": ping, "jitter": jitter, "ip": s("ip"), "isp": isp, "city": s("city"), "country": s("country"), "server": server_name, "mode": mode,
        "at": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64) }))
}
