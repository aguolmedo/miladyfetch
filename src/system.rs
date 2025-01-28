
use whoami;
use sysinfo::System;
use std::env;
use std::fs;

pub struct SystemInfo {
    pub hostname: String,
    pub username: String,
    pub total_memory: u64,
    pub used_memory: u64,
    pub system_name: String,
    pub kernel_version: String,
    pub cpu_info: String,
}

impl SystemInfo {

    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let kernel = fs::read_to_string("/proc/sys/kernel/osrelease")
            .unwrap_or_else(|_| "Unknown".to_string())
            .trim()
            .to_string();

        SystemInfo {
            hostname: whoami::fallible::hostname().unwrap(),
            username: whoami::username(),
            total_memory: sys.total_memory(),
            used_memory: sys.used_memory(),
            system_name: String::from(env::consts::OS),
            kernel_version: kernel,
            cpu_info: sys.cpus().first().map(|cpu| cpu.brand()).unwrap().to_string(),
        }
    }

}






