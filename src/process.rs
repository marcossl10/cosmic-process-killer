// SPDX-License-Identifier: MIT

use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

/// Result type for process operations with error context
pub type ProcessResult<T> = Result<T, ProcessError>;

/// Error types for process operations
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessError {
    /// Process not found
    NotFound,
    /// Permission denied
    PermissionDenied,
    /// Process is protected (system process)
    Protected(String),
    /// Signal sending failed
    SignalFailed(String),
    /// Unknown error
    Unknown(String),
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessError::NotFound => write!(f, "Process not found"),
            ProcessError::PermissionDenied => write!(f, "Permission denied"),
            ProcessError::Protected(name) => write!(f, "Protected process: {}", name),
            ProcessError::SignalFailed(msg) => write!(f, "Signal failed: {}", msg),
            ProcessError::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
    pub status: String,
    pub is_system: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Cpu,
    Memory,
    Pid,
    Name,
}

pub struct ProcessManager {
    system: System,
}

impl ProcessManager {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self { system }
    }

    pub fn refresh(&mut self) {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::everything(),
        );
    }

    pub fn get_processes(&mut self, sort_by: SortBy) -> Vec<ProcessInfo> {
        self.refresh();
        
        let mut processes: Vec<ProcessInfo> = self
            .system
            .processes()
            .iter()
            .map(|(pid, process)| {
                let name = process.name().to_string_lossy().to_string();
                let is_system = is_system_service(&name) || is_critical_process(&name);
                
                ProcessInfo {
                    pid: pid.as_u32(),
                    name,
                    cpu_usage: process.cpu_usage(),
                    memory: process.memory(),
                    status: format!("{:?}", process.status()),
                    is_system,
                }
            })
            .collect();

        // Sort
        match sort_by {
            SortBy::Cpu => processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap()),
            SortBy::Memory => processes.sort_by(|a, b| b.memory.cmp(&a.memory)),
            SortBy::Pid => processes.sort_by(|a, b| a.pid.cmp(&b.pid)),
            SortBy::Name => processes.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
        }
        
        processes
    }

    pub fn get_high_cpu_processes(&mut self, threshold: f32, sort_by: SortBy) -> Vec<ProcessInfo> {
        self.get_processes(sort_by)
            .into_iter()
            .filter(|p| p.cpu_usage > threshold)
            .collect()
    }

    pub fn get_process_by_pid(&mut self, pid: u32) -> Option<ProcessInfo> {
        self.refresh();
        
        self.system
            .processes()
            .iter()
            .find(|(p, _)| p.as_u32() == pid)
            .map(|(p, process)| {
                let name = process.name().to_string_lossy().to_string();
                let is_system = is_system_service(&name) || is_critical_process(&name);
                
                ProcessInfo {
                    pid: p.as_u32(),
                    name,
                    cpu_usage: process.cpu_usage(),
                    memory: process.memory(),
                    status: format!("{:?}", process.status()),
                    is_system,
                }
            })
    }

    /// Check if killing a process is allowed
    pub fn can_kill_process(&self, process: &ProcessInfo) -> ProcessResult<()> {
        // Check if process is a critical system process that should be protected
        if is_critical_process(&process.name) {
            return Err(ProcessError::Protected(process.name.clone()));
        }
        
        Ok(())
    }

    pub fn kill_process(&self, pid: u32) -> ProcessResult<()> {
        let nix_pid = Pid::from_raw(pid as i32);
        
        signal::kill(nix_pid, Signal::SIGTERM)
            .map_err(|e| ProcessError::SignalFailed(e.to_string()))?;
        
        Ok(())
    }

    pub fn force_kill_process(&self, pid: u32) -> ProcessResult<()> {
        let nix_pid = Pid::from_raw(pid as i32);
        
        signal::kill(nix_pid, Signal::SIGKILL)
            .map_err(|e| ProcessError::SignalFailed(e.to_string()))?;
        
        Ok(())
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a process name matches known system services
fn is_system_service(name: &str) -> bool {
    let system_services = [
        "systemd", "dbus-daemon", "dbus-broker", "NetworkManager",
        "containerd", "dockerd", "kubelet", "coredns",
        "sshd", "rsyslogd", "cron", "atd",
        "polkitd", "avahi-daemon", "cupsd", "bluetoothd",
        "firewalld", "iptables", "nftables",
        "systemd-resolved", "systemd-logind", "systemd-udevd",
    ];
    
    system_services.iter().any(|service| name.starts_with(service))
}

/// Check if a process is a critical system process that should be protected
fn is_critical_process(name: &str) -> bool {
    let critical_processes = [
        "init", "systemd", "kthreadd", "migration", "rcu_sched",
        "lru-add-drain", "watchdog", "cpuhp", "netns", "rcu_bh",
        "kasimer", "writeback", "kprobe", "khungtaskd", "oom_reaper",
        "ksmd", "khugepaged", "crypto", "kintegrityd", "kblockd",
        "edac-poller", "devfreq_wq", "watchdogd", "kswapd0",
    ];
    
    critical_processes.contains(&name)
}
