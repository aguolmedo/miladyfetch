use system::SystemInfo;
pub mod system;

fn main() {

    let ascii_art = include_str!("ascii.txt")
        .lines()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    let sys_info = SystemInfo::new();
    
    let system_name: String = format!("{}", sys_info.system_name);
    let kernel_version = format!("Kernel: {}", sys_info.kernel_version.replace("\"", ""));
    let memory = format!(
        "{} MB / {} MB",
        sys_info.used_memory / 1048576,
        sys_info.total_memory / 1048576
    );
     
     for (i, line) in ascii_art.iter().enumerate() {
        match i {
            5 => println!("{}   {}@{}", line, sys_info.username, sys_info.hostname),
            7 => println!("{}   OS: {}", line, system_name),
            8 => println!("{}   {}\x1B[K", line, kernel_version),
            9 => println!("{}   Memory: {}", line, memory),
            10 => println!("{}   CPU: {}\x1B[K", line, sys_info.cpu_info),
            11 => println!("{}   GPU: {}\x1B[K", line, sys_info.gpu_name),
            12 => println!("{}   Uptime: {}", line, sys_info.uptime),

            _ => println!("{}", line),
        }
    }


    
}




