use pollster;
use whoami;
use wgpu;
use sysinfo::{Disks, Networks, Pid, System};
use std::env;
use std::fs;
use std::net::IpAddr;
use std::path::Path;
use std::process::Command;

const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

const SHELLS: &[&str] = &["bash", "zsh", "fish", "sh", "dash", "ksh", "tcsh", "csh", "nu", "elvish", "xonsh"];
const WRAPPERS: &[&str] = &["sudo", "su", "doas", "tmux", "screen", "login", "miladyfetch", "cargo"];
const WMS: &[&str] = &[
    "dwm", "i3", "bspwm", "openbox", "awesome", "xmonad", "qtile", "herbstluftwm", "fluxbox",
    "icewm", "spectrwm", "leftwm", "berry", "xfwm4", "marco", "kwin_x11", "kwin_wayland",
    "mutter", "gnome-shell", "sway", "Hyprland", "river", "niri", "wayfire", "labwc", "dwl",
];

pub struct SystemInfo {
    pub hostname: String,
    pub username: String,
    pub os: String,
    pub host: Option<String>,
    pub kernel: String,
    pub uptime: String,
    pub packages: Option<String>,
    pub shell: Option<String>,
    pub displays: Vec<String>,
    pub wm: Option<String>,
    pub theme: Option<String>,
    pub icons: Option<String>,
    pub font: Option<String>,
    pub cursor: Option<String>,
    pub terminal: Option<String>,
    pub terminal_font: Option<String>,
    pub cpu: String,
    pub gpu: Option<String>,
    pub memory: String,
    pub swap: String,
    pub disk: Option<String>,
    pub local_ip: Option<String>,
    pub battery: Option<String>,
    pub locale: Option<String>,
}

impl SystemInfo {

    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let gtk = read_gtk_settings();
        let terminal_name = find_terminal(&sys);

        SystemInfo {
            hostname: whoami::fallible::hostname().unwrap_or_else(|_| "unknown".to_string()),
            username: whoami::username(),
            os: get_os(),
            host: get_host(),
            kernel: format!("Linux {}", read_trim("/proc/sys/kernel/osrelease").unwrap_or_default()),
            uptime: format_uptime(System::uptime()),
            packages: get_packages(),
            shell: get_shell(),
            displays: get_displays(),
            wm: get_wm(&sys),
            theme: gtk_value(&gtk, "gtk-theme-name").map(|v| format!("{} [GTK3]", v)),
            icons: gtk_value(&gtk, "gtk-icon-theme-name").map(|v| format!("{} [GTK3]", v)),
            font: gtk_value(&gtk, "gtk-font-name").map(|v| format_gtk_font(&v)),
            cursor: gtk_value(&gtk, "gtk-cursor-theme-name"),
            terminal: terminal_name.as_deref().map(get_terminal_version),
            terminal_font: terminal_name.as_deref().and_then(get_terminal_font),
            cpu: get_cpu(&sys),
            gpu: get_gpu(),
            memory: usage(sys.used_memory(), sys.total_memory()),
            swap: if sys.total_swap() == 0 { "Disabled".to_string() } else { usage(sys.used_swap(), sys.total_swap()) },
            disk: get_disk(),
            local_ip: get_local_ip(),
            battery: get_battery(),
            locale: env::var("LC_ALL").ok().filter(|s| !s.is_empty()).or_else(|| env::var("LANG").ok()),
        }
    }
}

fn read_trim<P: AsRef<Path>>(path: P) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

fn usage(used: u64, total: u64) -> String {
    let pct = if total == 0 { 0.0 } else { used as f64 / total as f64 * 100.0 };
    format!("{:.2} GiB / {:.2} GiB ({:.0}%)", used as f64 / GIB, total as f64 / GIB, pct)
}

fn get_os() -> String {
    let release = fs::read_to_string("/etc/os-release").unwrap_or_default();
    let name = release
        .lines()
        .find_map(|l| l.strip_prefix("PRETTY_NAME="))
        .map(|v| v.trim_matches('"').to_string())
        .unwrap_or_else(|| env::consts::OS.to_string());
    format!("{} {}", name, env::consts::ARCH)
}

fn get_host() -> Option<String> {
    let name = read_trim("/sys/class/dmi/id/product_name")?;
    match read_trim("/sys/class/dmi/id/product_version") {
        Some(v) if v != "None" && !v.contains("To be filled") => Some(format!("{} ({})", name, v)),
        _ => Some(name),
    }
}

fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = secs % 86400 / 3600;
    let mins = secs % 3600 / 60;
    let plural = |n: u64, unit: &str| format!("{} {}{}", n, unit, if n == 1 { "" } else { "s" });

    let mut parts = Vec::new();
    if days > 0 { parts.push(plural(days, "day")); }
    if hours > 0 { parts.push(plural(hours, "hour")); }
    if mins > 0 || parts.is_empty() { parts.push(plural(mins, "min")); }
    parts.join(", ")
}

fn get_packages() -> Option<String> {
    let mut counts = Vec::new();

    if let Ok(entries) = fs::read_dir("/var/lib/pacman/local") {
        let n = entries.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).count();
        counts.push(format!("{} (pacman)", n));
    }
    if let Ok(status) = fs::read_to_string("/var/lib/dpkg/status") {
        let n = status.lines().filter(|l| *l == "Status: install ok installed").count();
        counts.push(format!("{} (dpkg)", n));
    }
    if let Ok(entries) = fs::read_dir("/var/lib/flatpak/app") {
        let n = entries.count();
        if n > 0 { counts.push(format!("{} (flatpak)", n)); }
    }

    if counts.is_empty() { None } else { Some(counts.join(", ")) }
}

fn parse_version(text: &str) -> Option<String> {
    text.lines().next()?
        .split_whitespace()
        .find(|w| w.chars().next().map_or(false, |c| c.is_ascii_digit()) && w.contains('.'))
        .map(|w| w.split(|c: char| !(c.is_ascii_digit() || c == '.')).next().unwrap_or(w).to_string())
}

fn get_shell() -> Option<String> {
    let path = env::var("SHELL").ok()?;
    let name = Path::new(&path).file_name()?.to_string_lossy().to_string();
    match run(&path, &["--version"]).and_then(|v| parse_version(&v)) {
        Some(version) => Some(format!("{} {}", name, version)),
        None => Some(name),
    }
}

fn session_type() -> &'static str {
    if env::var("WAYLAND_DISPLAY").is_ok() || env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland") {
        "Wayland"
    } else {
        "X11"
    }
}

fn get_wm(sys: &System) -> Option<String> {
    let name = sys
        .processes()
        .values()
        .map(|p| p.name().to_string_lossy().to_string())
        .find(|n| WMS.contains(&n.as_str()))
        .or_else(|| env::var("XDG_CURRENT_DESKTOP").ok())?;
    Some(format!("{} ({})", name, session_type()))
}

fn read_gtk_settings() -> String {
    let home = env::var("HOME").unwrap_or_default();
    let config = env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{}/.config", home));
    fs::read_to_string(format!("{}/gtk-3.0/settings.ini", config))
        .or_else(|_| fs::read_to_string("/etc/gtk-3.0/settings.ini"))
        .unwrap_or_default()
}

fn gtk_value(settings: &str, key: &str) -> Option<String> {
    settings.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
    })
}

fn format_gtk_font(font: &str) -> String {
    match font.rsplit_once(' ') {
        Some((family, size)) if size.parse::<f32>().is_ok() => format!("{} ({}pt) [GTK3]", family, size),
        _ => format!("{} [GTK3]", font),
    }
}

fn find_terminal(sys: &System) -> Option<String> {
    let mut pid: Pid = sysinfo::get_current_pid().ok()?;
    while let Some(process) = sys.process(pid) {
        let name = process.name().to_string_lossy().to_string();
        let skip = SHELLS.contains(&name.as_str()) || WRAPPERS.contains(&name.as_str()) || name.starts_with("claude");
        if !skip && name != "systemd" && name != "init" {
            return Some(name);
        }
        pid = process.parent()?;
    }
    env::var("TERM_PROGRAM").ok().or_else(|| env::var("TERM").ok())
}

fn get_terminal_version(name: &str) -> String {
    match run(name, &["--version"]).and_then(|v| parse_version(&v)) {
        Some(version) => format!("{} {}", name, version),
        None => name.to_string(),
    }
}

fn get_terminal_font(name: &str) -> Option<String> {
    if name != "alacritty" {
        return None;
    }
    let home = env::var("HOME").ok()?;
    let config = env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{}/.config", home));
    let text = fs::read_to_string(format!("{}/alacritty/alacritty.toml", config)).unwrap_or_default();

    let mut section = String::new();
    let (mut family, mut style, mut size) = (None, None, None);
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            section = line.trim_matches(|c| c == '[' || c == ']').to_string();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim().trim_matches('"').to_string());
        match (section.as_str(), k) {
            ("font", "size") => size = Some(v),
            ("font", "normal.family") | ("font.normal", "family") => family = Some(v),
            ("font", "normal.style") | ("font.normal", "style") => style = Some(v),
            _ => {}
        }
    }

    let size: f32 = size.and_then(|s| s.parse().ok()).unwrap_or(11.25);
    Some(format!(
        "{} ({:.1}pt, {})",
        family.unwrap_or_else(|| "monospace".to_string()),
        size,
        style.unwrap_or_else(|| "Regular".to_string())
    ))
}

fn count_cpu_list(path: &str) -> Option<usize> {
    let list = read_trim(path)?;
    Some(list.split(',').map(|range| match range.split_once('-') {
        Some((a, b)) => b.parse::<usize>().unwrap_or(0) - a.parse::<usize>().unwrap_or(0) + 1,
        None => 1,
    }).sum())
}

fn get_cpu(sys: &System) -> String {
    let brand = sys.cpus().first().map(|cpu| cpu.brand().trim().to_string()).unwrap_or_default();

    let hybrid: Vec<String> = ["cpu_core", "cpu_atom", "cpu_lowpower"]
        .iter()
        .filter_map(|t| count_cpu_list(&format!("/sys/devices/{}/cpus", t)))
        .map(|n| n.to_string())
        .collect();
    let cores = if hybrid.len() > 1 { hybrid.join("+") } else { sys.cpus().len().to_string() };

    let max_khz = fs::read_dir("/sys/devices/system/cpu")
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| read_trim(e.path().join("cpufreq/cpuinfo_max_freq")))
        .filter_map(|s| s.parse::<u64>().ok())
        .max();

    match max_khz {
        Some(khz) => format!("{} ({}) @ {:.2} GHz", brand, cores, khz as f64 / 1_000_000.0),
        None => format!("{} ({})", brand, cores),
    }
}

fn get_gpu() -> Option<String> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))?;
    let info = adapter.get_info();

    let name = pci_gpu_name().unwrap_or_else(|| {
        let mut name = info.name.replace("(R)", "").replace("(TM)", "");
        if let Some(i) = name.find(" (") { name.truncate(i); }
        name.trim().to_string()
    });

    let freq = fs::read_dir("/sys/class/drm")
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| read_trim(e.path().join("gt_max_freq_mhz"))
            .or_else(|| read_trim(e.path().join("device/tile0/gt0/freq0/max_freq"))))
        .filter_map(|s| s.parse::<f64>().ok())
        .next()
        .map(|mhz| format!(" @ {:.2} GHz", mhz / 1000.0))
        .unwrap_or_default();

    let kind = match info.device_type {
        wgpu::DeviceType::IntegratedGpu => " [Integrated]",
        wgpu::DeviceType::DiscreteGpu => " [Discrete]",
        wgpu::DeviceType::VirtualGpu => " [Virtual]",
        _ => "",
    };
    Some(format!("{}{}{}", name, freq, kind))
}

fn pci_gpu_name() -> Option<String> {
    let card = fs::read_dir("/sys/class/drm").ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.file_name().map_or(false, |n| { let n = n.to_string_lossy(); n.starts_with("card") && !n.contains('-') }))?;
    let vendor = read_trim(card.join("device/vendor"))?.trim_start_matches("0x").to_lowercase();
    let device = read_trim(card.join("device/device"))?.trim_start_matches("0x").to_lowercase();

    let ids = ["/usr/share/hwdata/pci.ids", "/usr/share/misc/pci.ids"]
        .iter()
        .find_map(|p| fs::read_to_string(p).ok())?;

    let mut vendor_name = None;
    for line in ids.lines() {
        if vendor_name.is_none() {
            if let Some(rest) = line.strip_prefix(&format!("{}  ", vendor)) { vendor_name = Some(rest.to_string()); }
            continue;
        }
        if line.starts_with('#') || line.is_empty() { continue; }
        if !line.starts_with('\t') { break; }
        if let Some(rest) = line.strip_prefix(&format!("\t{}  ", device)) {
            let short_vendor = match vendor.as_str() {
                "8086" => "Intel".to_string(),
                "10de" => "NVIDIA".to_string(),
                "1002" => "AMD".to_string(),
                _ => vendor_name.unwrap_or_default(),
            };
            let model = match (rest.rfind('['), rest.rfind(']')) {
                (Some(a), Some(b)) if a < b => &rest[a + 1..b],
                _ => rest,
            };
            return Some(format!("{} {}", short_vendor, model));
        }
    }
    None
}

fn edid_name(edid: &[u8]) -> Option<String> {
    if edid.len() < 128 {
        return None;
    }
    for d in edid[54..126].chunks(18) {
        if d[0] == 0 && d[1] == 0 && d[3] == 0xFC {
            let name = String::from_utf8_lossy(&d[5..18]).trim().to_string();
            if !name.is_empty() { return Some(name); }
        }
    }
    let m = u16::from_be_bytes([edid[8], edid[9]]);
    let letter = |shift: u16| (((m >> shift) & 0x1F) as u8 + b'A' - 1) as char;
    let product = u16::from_le_bytes([edid[10], edid[11]]);
    Some(format!("{}{}{}{:04X}", letter(10), letter(5), letter(0), product))
}

fn get_displays() -> Vec<String> {
    let Some(output) = run("xrandr", &["--props"]) else { return Vec::new() };
    let scale = run("xrdb", &["-query"])
        .and_then(|s| s.lines().find_map(|l| l.strip_prefix("Xft.dpi:").map(|v| v.trim().to_string())))
        .and_then(|v| v.parse::<f64>().ok())
        .map(|dpi| dpi / 96.0)
        .filter(|s| *s > 1.0)
        .map(|s| format!(" @ {}x", s))
        .unwrap_or_default();

    struct Out { connector: String, primary: bool, res: String, inches: Option<f64>, refresh: Option<f64>, edid: Vec<u8> }
    let mut outs: Vec<Out> = Vec::new();
    let mut in_edid = false;

    for line in output.lines() {
        if !line.starts_with(char::is_whitespace) {
            in_edid = false;
            let words: Vec<&str> = line.split_whitespace().collect();
            if words.get(1) != Some(&"connected") { continue; }
            let Some(geom) = words.iter().find(|w| w.contains('x') && w.contains('+')) else { continue };
            let mm: Vec<f64> = words.iter().filter_map(|w| w.strip_suffix("mm")?.parse().ok()).collect();
            outs.push(Out {
                connector: words[0].to_string(),
                primary: words.contains(&"primary"),
                res: geom.split('+').next().unwrap_or("").to_string(),
                inches: (mm.len() == 2 && mm[0] > 0.0).then(|| (mm[0].powi(2) + mm[1].powi(2)).sqrt() / 25.4),
                refresh: None,
                edid: Vec::new(),
            });
        } else if let Some(out) = outs.last_mut() {
            let t = line.trim();
            if t.starts_with("EDID:") {
                in_edid = true;
            } else if in_edid && t.len() == 32 && t.chars().all(|c| c.is_ascii_hexdigit()) {
                out.edid.extend((0..32).step_by(2).filter_map(|i| u8::from_str_radix(&t[i..i + 2], 16).ok()));
            } else {
                in_edid = false;
                if out.refresh.is_none() && t.contains('*') {
                    out.refresh = t.split_whitespace().find(|w| w.contains('*')).and_then(|w| w.trim_end_matches(['*', '+']).parse().ok());
                }
            }
        }
    }

    outs.iter().map(|o| {
        let name = edid_name(&o.edid).unwrap_or_else(|| o.connector.clone());
        let builtin = ["eDP", "LVDS", "DSI"].iter().any(|p| o.connector.starts_with(p));
        let mut s = format!("Display ({}): {}{}", name, o.res, scale);
        if let Some(i) = o.inches { s += &format!(" in {:.0}\"", i); }
        if let Some(r) = o.refresh { s += &format!(", {:.0} Hz", r); }
        s += if builtin { " [Built-in]" } else { " [External]" };
        if o.primary { s += " *"; }
        s
    }).collect()
}

fn get_disk() -> Option<String> {
    if let Some(out) = run("df", &["-B1", "--output=size,used,fstype", "/"]) {
        let cols: Vec<&str> = out.lines().nth(1).unwrap_or("").split_whitespace().collect();
        if let [size, used, fstype] = cols[..] {
            if let (Ok(size), Ok(used)) = (size.parse(), used.parse()) {
                return Some(format!("{} - {}", usage(used, size), fstype));
            }
        }
    }
    let disks = Disks::new_with_refreshed_list();
    let root = disks.iter().find(|d| d.mount_point() == Path::new("/"))?;
    let total = root.total_space();
    let used = total - root.available_space();
    Some(format!("{} - {}", usage(used, total), root.file_system().to_string_lossy()))
}

fn get_local_ip() -> Option<String> {
    let networks = Networks::new_with_refreshed_list();
    let mut candidates: Vec<(String, String)> = networks
        .iter()
        .filter(|(name, _)| *name != "lo" && !name.starts_with("docker") && !name.starts_with("br-") && !name.starts_with("veth"))
        .flat_map(|(name, data)| {
            data.ip_networks().iter().filter_map(move |ip| match ip.addr {
                IpAddr::V4(v4) if !v4.is_loopback() => Some((name.clone(), format!("{}/{}", v4, ip.prefix))),
                _ => None,
            })
        })
        .collect();
    candidates.sort();
    candidates.into_iter().next().map(|(iface, ip)| format!("Local IP ({}): {}", iface, ip))
}

fn get_battery() -> Option<String> {
    let bat = fs::read_dir("/sys/class/power_supply")
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| read_trim(p.join("type")).as_deref() == Some("Battery"))?;
    let model = read_trim(bat.join("model_name")).unwrap_or_else(|| "Battery".to_string());
    let capacity = read_trim(bat.join("capacity"))?;
    let status = read_trim(bat.join("status")).map(|s| format!(" [{}]", s)).unwrap_or_default();
    Some(format!("Battery ({}): {}%{}", model, capacity, status))
}
