
use pollster;
use whoami;
use wgpu;
use sysinfo::System;
use std::env;
use std::fs;
use std::io::Read;
use crate::system::fs::File;
use std::time::Duration;

pub struct SystemInfo {
    pub hostname: String,
    pub username: String,
    pub total_memory: u64,
    pub used_memory: u64,
    pub system_name: String,
    pub kernel_version: String,
    pub cpu_info: String,
    pub gpu_name: String,
    pub uptime: String
}

impl SystemInfo {

    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();


        let kernel = fs::read_to_string("/proc/sys/kernel/osrelease")
            .unwrap_or_else(|_| "Unknown".to_string())
            .trim()
            .to_string();

        let instance = wgpu::Instance::default();

        // Request an adapter
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("Failed to find an appropriate adapter");
        

        
        
        SystemInfo {
            hostname: whoami::fallible::hostname().unwrap(),
            username: whoami::username(),
            total_memory: sys.total_memory(),
            used_memory: sys.used_memory(),
            system_name: String::from(env::consts::OS),
            kernel_version: kernel,
            cpu_info: sys.cpus().first().map(|cpu| cpu.brand()).unwrap().to_string(),
            gpu_name: adapter.get_info().name.split("(").next().unwrap().to_string(),
            uptime: Self::get_uptime().unwrap(),
        
        }

    }

    fn get_uptime() -> std::io::Result<String> {
        let mut file = File::open("/proc/uptime")?;
        let mut c = String::new();
        file.read_to_string(&mut c)?;
        let uptime = Duration::from_secs_f32(
            c.split_whitespace()
                .next()
                .unwrap_or("0")
                .parse::<f32>()
                .unwrap_or(0.0),
        );
        let h = uptime.as_secs() / 3600;
        let m = (uptime.as_secs() - h * 3600) / 60;
        let  uptime_string = format!("{} hours, {} mins", h.to_string(),m.to_string());




        Ok(uptime_string)
    }



}