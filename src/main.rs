use system::SystemInfo;
pub mod system;

fn main() {

    let ascii_art = include_str!("ascii.txt")
        .lines()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    let sys_info = SystemInfo::new();
    
    let system_name: String = format!("{}", sys_info.system_name.unwrap());
    let kernel_version = format!("Kernel: {}", sys_info.kernel_version.unwrap_or("Unknown".to_string()).replace("\"", ""));
    let memory = format!(
        "{} MB / {} MB",
        sys_info.used_memory / 1048576,
        sys_info.total_memory / 1048576
    );





     // Print side-by-side ASCII art and system info
     for (i, line) in ascii_art.iter().enumerate() {
        match i {
            5 => println!("{}   {}@{}", line, sys_info.username, sys_info.hostname),
            7 => println!("{}   OS: {}", line, system_name),
            8 => println!("{}   {}", line, kernel_version),
            9 => println!("{}   Memory: {}", line, memory),
            10 => println!("{}   CPU: {}", line, sys_info.cpu_info),

            _ => println!("{}", line),
        }
    }


    
}




