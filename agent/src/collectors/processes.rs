use serde::Serialize;
use std::collections::HashMap;
use sysinfo::System;

use crate::types::{Bytes, Percent, UnixTs};

const TOP_N: usize = 10;

#[derive(Debug, Serialize, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub exe_path: Option<String>,
    pub cmd: Vec<String>,
    pub cpu_usage: Percent,
    pub memory_bytes: Bytes,
    pub memory_percent: Percent,
    pub status: String,
    pub user: Option<String>,
    pub start_time: UnixTs,
    pub threads: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: Option<String>,
    pub status: String,
    pub process_count: usize,
    pub total_cpu: Percent,
    pub total_memory: Bytes,
    pub pids: Vec<u32>,
    pub exe_path: Option<String>,
    pub command: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProcessSummary {
    pub total_processes: usize,
    pub running_processes: usize,
    pub sleeping_processes: usize,
    pub zombie_processes: usize,
    pub total_threads: usize,
    pub top_cpu_processes: Vec<ProcessInfo>,
    pub top_memory_processes: Vec<ProcessInfo>,
    pub detected_services: Vec<ServiceInfo>,
}

// TODO-Agregar agregar más servicios conforme se avance
const SVC_MAP: &[(&str, &str)] = &[
    ("nginx", "Nginx"), ("apache2", "Apache"), ("httpd", "Apache"),
    ("postgres", "PostgreSQL"), ("mysqld", "MySQL"), ("mariadb", "MariaDB"),
    ("mongod", "MongoDB"), ("redis", "Redis"),
    ("node", "Node.js"), ("java", "Java"), ("python", "Python"),
    ("dockerd", "Docker"), ("containerd", "containerd"),
    ("prometheus", "Prometheus"), ("grafana", "Grafana"),
    ("rabbitmq", "RabbitMQ"), ("kafka", "Kafka"), ("nats-server", "NATS"),
];

pub struct ProcessCollector;

impl ProcessCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn collect(&mut self) -> ProcessSummary {
        let mut sys = System::new_all();
        sys.refresh_all();
        
        let total_mem = sys.total_memory();
        let proc_count = sys.processes().len();
        
        // Usar índices para evitar clonar todo el vector
        let mut cpu_indices: Vec<usize> = Vec::with_capacity(proc_count);
        let mut mem_indices: Vec<usize> = Vec::with_capacity(proc_count);
        let mut processes = Vec::with_capacity(proc_count);
        
        let mut running = 0usize;
        let mut sleeping = 0usize;
        let mut zombie = 0usize;

        for (idx, (pid, proc)) in sys.processes().iter().enumerate() {
            let status_str = format!("{:?}", proc.status());
            match status_str.as_str() {
                "Run" | "Running" => running += 1,
                "Sleep" | "Sleeping" | "Idle" => sleeping += 1,
                "Zombie" => zombie += 1,
                _ => {}
            }
            
            let mem_pct = if total_mem > 0 {
                (proc.memory() as f64 / total_mem as f64 * 100.0) as Percent
            } else {
                0.0
            };
            
            processes.push(ProcessInfo {
                pid: pid.as_u32(),
                name: proc.name().to_string_lossy().into_owned(),
                exe_path: proc.exe().map(|p| p.to_string_lossy().into_owned()),
                cmd: proc.cmd().iter().map(|s| s.to_string_lossy().into_owned()).collect(),
                cpu_usage: proc.cpu_usage(),
                memory_bytes: proc.memory(),
                memory_percent: mem_pct,
                status: status_str,
                user: proc.user_id().map(|u| format!("{:?}", u)),
                start_time: proc.start_time() as UnixTs,
                threads: 0,
            });
            
            cpu_indices.push(idx);
            mem_indices.push(idx);
        }

        // Ordenar índices por CPU (descendente) usando total_cmp para evitar unwrap
        cpu_indices.sort_by(|&a, &b| {
            processes[b].cpu_usage.total_cmp(&processes[a].cpu_usage)
        });
        
        // Ordenar índices por memoria (descendente)
        mem_indices.sort_by(|&a, &b| {
            processes[b].memory_bytes.cmp(&processes[a].memory_bytes)
        });

        // Recolectar top N sin clonar todo el vector
        let top_cpu: Vec<_> = cpu_indices.iter()
            .take(TOP_N)
            .map(|&i| processes[i].clone())
            .collect();
        
        let top_mem: Vec<_> = mem_indices.iter()
            .take(TOP_N)
            .map(|&i| processes[i].clone())
            .collect();

        // Detectar servicios conocidos
        let services = Self::find_services(&processes);

        ProcessSummary {
            total_processes: processes.len(),
            running_processes: running,
            sleeping_processes: sleeping,
            zombie_processes: zombie,
            total_threads: 0,
            top_cpu_processes: top_cpu,
            top_memory_processes: top_mem,
            detected_services: services,
        }
    }

    fn find_services(processes: &[ProcessInfo]) -> Vec<ServiceInfo> {
        let mut found: HashMap<&str, ServiceInfo> = HashMap::new();
        
        for proc in processes {
            let name_lower = proc.name.to_lowercase();
            let exe_lower = proc.exe_path.as_deref()
                .map(str::to_lowercase)
                .unwrap_or_default();
            
            for &(pat, label) in SVC_MAP {
                if !name_lower.contains(pat) && !exe_lower.contains(pat) {
                    continue;
                }
                
                let entry = found.entry(pat).or_insert_with(|| ServiceInfo {
                    name: pat.to_owned(),
                    display_name: Some(label.to_owned()),
                    status: "running".to_owned(),
                    process_count: 0,
                    total_cpu: 0.0,
                    total_memory: 0,
                    pids: Vec::new(),
                    exe_path: proc.exe_path.clone(),
                    command: proc.cmd.first().cloned(),
                });
                
                entry.process_count += 1;
                entry.total_cpu += proc.cpu_usage;
                entry.total_memory += proc.memory_bytes;
                entry.pids.push(proc.pid);
                break;
            }
        }
        
        // Ordenar por uso de CPU (los más activos primero)
        let mut result: Vec<_> = found.into_values().collect();
        result.sort_by(|a, b| b.total_cpu.total_cmp(&a.total_cpu));
        
        result
    }
}

impl Default for ProcessCollector {
    fn default() -> Self {
        Self::new()
    }
}
