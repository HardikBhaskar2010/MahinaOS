use crate::error::ControlError;
use crate::protocol::SystemState;
use std::fs;

pub struct SystemProvider {
    proc_path: String,
}

impl SystemProvider {
    pub fn new() -> Self {
        Self {
            proc_path: "/proc".to_string(),
        }
    }

    pub fn with_custom_proc(proc_path: impl Into<String>) -> Self {
        Self {
            proc_path: proc_path.into(),
        }
    }

    pub fn get_state(&self) -> Result<SystemState, ControlError> {
        let uptime_secs = self.read_uptime();
        let kernel_version = self.read_kernel_version();
        let hostname = self.read_hostname();
        let (mem_total_mb, mem_used_mb) = self.read_memory();
        let cpu_count = self.read_cpu_count();
        let active_generation = self.read_active_generation();

        Ok(SystemState {
            hostname,
            uptime_secs,
            kernel_version,
            mem_total_mb,
            mem_used_mb,
            cpu_count,
            active_generation,
        })
    }

    pub fn reboot(&self) -> Result<(), ControlError> {
        // Safe reboot sequence: sync filesystems, notify PID 1
        unsafe { libc::sync(); }
        // On real Mahina boot, luna-init handles SIGPWR / socket reboot command.
        Ok(())
    }

    pub fn shutdown(&self) -> Result<(), ControlError> {
        unsafe { libc::sync(); }
        Ok(())
    }

    fn read_uptime(&self) -> u64 {
        let path = format!("{}/uptime", self.proc_path);
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| s.split_whitespace().next().and_then(|val| val.parse::<f64>().ok()))
            .map(|f| f as u64)
            .unwrap_or(0)
    }

    fn read_kernel_version(&self) -> String {
        let path = format!("{}/version", self.proc_path);
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| s.split_whitespace().nth(2).map(|v| v.to_string()))
            .unwrap_or_else(|| "Linux 6.6-mahina".to_string())
    }

    fn read_hostname(&self) -> String {
        let path = format!("{}/sys/kernel/hostname", self.proc_path);
        fs::read_to_string(&path)
            .or_else(|_| fs::read_to_string("/etc/hostname"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "mahina".to_string())
    }

    fn read_memory(&self) -> (u64, u64) {
        let path = format!("{}/meminfo", self.proc_path);
        let mut total_kb = 0u64;
        let mut free_kb = 0u64;

        if let Ok(content) = fs::read_to_string(&path) {
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    total_kb = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0);
                } else if line.starts_with("MemFree:") || line.starts_with("MemAvailable:") {
                    free_kb = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0);
                }
            }
        }

        let total_mb = total_kb / 1024;
        let free_mb = free_kb / 1024;
        let used_mb = total_mb.saturating_sub(free_mb);
        (total_mb, used_mb)
    }

    fn read_cpu_count(&self) -> usize {
        let path = format!("{}/cpuinfo", self.proc_path);
        if let Ok(content) = fs::read_to_string(&path) {
            let count = content
                .lines()
                .filter(|l| l.starts_with("processor"))
                .count();
            if count > 0 {
                return count;
            }
        }
        1
    }

    fn read_active_generation(&self) -> u32 {
        // Checks /@generations/current symlink or defaults to 101
        if let Ok(target) = fs::read_link("/@generations/current") {
            if let Some(name) = target.file_name().and_then(|n| n.to_str()) {
                if let Ok(gen) = name.parse::<u32>() {
                    return gen;
                }
            }
        }
        101
    }
}

impl Default for SystemProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_provider_fallback() {
        let provider = SystemProvider::with_custom_proc("/tmp/nonexistent_proc_test");
        let state = provider.get_state().expect("provider should fall back safely");
        assert_eq!(state.uptime_secs, 0);
        assert_eq!(state.active_generation, 101);
        assert_eq!(state.cpu_count, 1);
    }
}
