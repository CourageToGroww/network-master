use sysinfo::System;

pub struct SystemInfo {
    sys: System,
}

impl SystemInfo {
    pub fn new() -> Self {
        Self {
            sys: System::new_all(),
        }
    }

    pub fn refresh(&mut self) {
        self.sys.refresh_all();
    }

    pub fn cpu_usage(&self) -> f32 {
        self.sys.global_cpu_usage()
    }

    pub fn memory_usage_mb(&self) -> u32 {
        (self.sys.used_memory() / 1_048_576) as u32
    }

    pub fn hostname() -> String {
        System::host_name().unwrap_or_else(|| "unknown".to_string())
    }

    pub fn os_info() -> String {
        format!(
            "{} {}",
            System::name().unwrap_or_default(),
            System::os_version().unwrap_or_default()
        )
    }
}
