use system::SystemInfo;
pub mod animation;
pub mod system;

fn main() {

    let sys_info = SystemInfo::new();

    let title = format!("{}@{}", sys_info.username, sys_info.hostname);
    let mut info = vec![title.clone(), "-".repeat(title.len())];

    let field = |label: &str, value: Option<String>| value.map(|v| format!("{}: {}", label, v));
    info.extend(field("OS", Some(sys_info.os)));
    info.extend(field("Host", sys_info.host));
    info.extend(field("Kernel", Some(sys_info.kernel)));
    info.extend(field("Uptime", Some(sys_info.uptime)));
    info.extend(field("Packages", sys_info.packages));
    info.extend(field("Shell", sys_info.shell));
    info.extend(sys_info.displays);
    info.extend(field("WM", sys_info.wm));
    info.extend(field("Theme", sys_info.theme));
    info.extend(field("Icons", sys_info.icons));
    info.extend(field("Font", sys_info.font));
    info.extend(field("Cursor", sys_info.cursor));
    info.extend(field("Terminal", sys_info.terminal));
    info.extend(field("Terminal Font", sys_info.terminal_font));
    info.extend(field("CPU", Some(sys_info.cpu)));
    info.extend(field("GPU", sys_info.gpu));
    info.extend(field("Memory", Some(sys_info.memory)));
    info.extend(field("Swap", Some(sys_info.swap)));
    info.extend(field("Disk (/)", sys_info.disk));
    info.extend(sys_info.local_ip);
    info.extend(sys_info.battery);
    info.extend(field("Locale", sys_info.locale));

    animation::run(&info);
}
