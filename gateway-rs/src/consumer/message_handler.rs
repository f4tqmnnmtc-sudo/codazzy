use crate::error::{AppError, Result};
use crate::models::metrics::*;
use crate::processing::InfluxWriter;
use crate::services::ServerProcessesService;
use lz4_flex::decompress_size_prepended;
use rmpv::Value;
use std::collections::HashMap;
use std::sync::Arc;

macro_rules! map_f { ($s:expr,$m:expr,$o:expr,{$($k:literal=>$f:ident:$c:ident),*$(,)?}) => { for(k,v)in $m{match $s.str(k).as_str(){$($k=>$o.$f=$s.$c(v),)*_=>{}}} }; ($s:expr,$m:expr,$o:expr,{$($k:literal=>$f:ident:$c:ident),*$(,)?},opt{$($ok:literal=>$of:ident:$oc:ident),*$(,)?}) => { for(k,v)in $m{match $s.str(k).as_str(){$($k=>$o.$f=$s.$c(v),)*$($ok=>$o.$of=Some($s.$oc(v)),)*_=>{}}} }; }
macro_rules! arr_f { ($s:expr,$a:expr,$o:expr,[$($i:literal=>$f:ident:$c:ident),*$(,)?]) => { $(if let Some(x)=$a.get($i){$o.$f=$s.$c(x);})*}; }

pub struct MessageHandler { writer: Arc<InfluxWriter>, procs: Option<Arc<ServerProcessesService>> }

impl MessageHandler {
    pub fn new(w: Arc<InfluxWriter>) -> Self { Self { writer: w, procs: None } }
    pub fn with_processes_service(w: Arc<InfluxWriter>, p: Arc<ServerProcessesService>) -> Self { Self { writer: w, procs: Some(p) } }

    pub async fn handle_message(&self, payload: &[u8]) -> Result<()> {
        let m = self.decode(payload)?; self.writer.write_metrics(&m).await?;
        if let (Some(svc), Some(ref p)) = (&self.procs, &m.processes) { if !p.top_cpu_processes.is_empty() || !p.detected_services.is_empty() { let _ = svc.save_processes(&m.node_id, p).await; } }
        Ok(())
    }

    fn decode(&self, payload: &[u8]) -> Result<SystemMetrics> {
        let d = decompress_size_prepended(payload).unwrap_or_else(|_| payload.to_vec());
        self.parse(&rmpv::decode::read_value(&mut &d[..]).map_err(|e| AppError::MessagePack(format!("decode: {e}")))?)
    }

    fn parse(&self, v: &Value) -> Result<SystemMetrics> { match v { Value::Map(m) => self.parse_map(m), Value::Array(a) => self.parse_arr(a), _ => Err(AppError::MessagePack("unexpected format".into())) } }

    fn parse_map(&self, map: &[(Value, Value)]) -> Result<SystemMetrics> {
        let mut m = SystemMetrics::default();
        for (k, v) in map { match self.str(k).as_str() { "node_id" => m.node_id = self.str(v), "timestamp" => m.timestamp = self.i64(v), "hardware" => m.hardware = self.hw(v)?, "network" => m.network = self.net(v)?, "storage" => m.storage = self.stor(v)?, "processes" => m.processes = self.procs_sum(v).ok(), "source_type" => m.source_type = Some(self.str(v)), "device_type" => m.device_type = Some(self.str(v)), "connection_protocol" => m.connection_protocol = Some(self.str(v)), "location" => m.location = Some(self.str(v)), "teleco_specific" => m.teleco_specific = self.teleco(v).ok(), _ => {} } }
        Ok(m)
    }

    fn parse_arr(&self, a: &[Value]) -> Result<SystemMetrics> {
        if a.len() < 5 { return Err(AppError::MessagePack(format!("array too short: {}", a.len()))) }
        Ok(SystemMetrics { node_id: self.str(&a[0]), timestamp: self.i64(&a[1]), hardware: self.hw(&a[2])?, network: self.net(&a[3])?, storage: self.stor(&a[4])?, processes: a.get(5).and_then(|v| self.procs_sum(v).ok()), ..Default::default() })
    }

    fn hw(&self, v: &Value) -> Result<HardwareMetrics> {
        let mut h = HardwareMetrics::default();
        match v { Value::Map(m) => { for (k, val) in m { match self.str(k).as_str() { "cpu_usage" => h.cpu_usage = self.f64_vec(val), "memory_usage" => h.memory_usage = self.mem(val)?, "thermal_sensors" => h.thermal_sensors = self.sensor_vec(val), _ => {} } } }
            Value::Array(a) => { if let Some(v) = a.first() { h.cpu_usage = self.f64_vec(v); } if let Some(v) = a.get(1) { h.memory_usage = self.mem(v)?; } if let Some(v) = a.get(2) { h.thermal_sensors = self.sensor_vec(v); } } _ => {} }
        Ok(h)
    }

    fn mem(&self, v: &Value) -> Result<MemoryUsage> {
        let mut m = MemoryUsage::default();
        match v { Value::Map(map) => { map_f!(self, map, m, { "total" => total: u64, "used" => used: u64, "available" => available: u64, "cached" => cached: u64, "buffers" => buffers: u64 }); }
            Value::Array(a) => { arr_f!(self, a, m, [0 => total: u64, 1 => used: u64, 2 => available: u64, 3 => cached: u64, 4 => buffers: u64]); } _ => {} }
        Ok(m)
    }

    fn sensor(&self, v: &Value) -> Result<ThermalSensor> {
        let (mut nm, mut tp, mut cr) = (String::new(), 0.0, None);
        match v { Value::Map(m) => { for (k, val) in m { match self.str(k).as_str() { "name" => nm = self.str(val), "temperature" => tp = self.f64(val), "critical_temp" => cr = Some(self.f64(val)), _ => {} } } }
            Value::Array(a) => { if let Some(x) = a.first() { nm = self.str(x); } if let Some(x) = a.get(1) { tp = self.f64(x); } if let Some(x) = a.get(2) { cr = Some(self.f64(x)); } } _ => {} }
        Ok(ThermalSensor { name: nm, temperature: tp, critical_temp: cr })
    }

    fn net(&self, v: &Value) -> Result<NetworkMetrics> {
        let ifaces = match v { Value::Map(m) => m.iter().find(|(k, _)| self.str(k) == "interfaces").and_then(|(_, x)| self.arr(x)), Value::Array(a) => if a.len() == 1 { self.arr(&a[0]).or_else(|| Some(a.clone())) } else { Some(a.clone()) }, _ => None };
        Ok(NetworkMetrics { interfaces: ifaces.map(|l| l.iter().filter_map(|x| self.iface(x).ok()).collect()).unwrap_or_default() })
    }

    fn iface(&self, v: &Value) -> Result<NetworkInterface> {
        let mut i = NetworkInterface { name: String::new(), bytes_sent: 0, bytes_received: 0, packets_sent: 0, packets_received: 0, errors_in: 0, errors_out: 0, is_up: true };
        match v { Value::Map(m) => { for (k, val) in m { match self.str(k).as_str() { "name" => i.name = self.str(val), "bytes_sent" => i.bytes_sent = self.u64(val), "bytes_received" => i.bytes_received = self.u64(val), "packets_sent" => i.packets_sent = self.u64(val), "packets_received" => i.packets_received = self.u64(val), "errors_in" => i.errors_in = self.u64(val), "errors_out" => i.errors_out = self.u64(val), "is_up" => i.is_up = self.bool(val), _ => {} } } }
            Value::Array(a) => { arr_f!(self, a, i, [0 => name: str, 1 => bytes_sent: u64, 2 => bytes_received: u64, 3 => packets_sent: u64, 4 => packets_received: u64, 5 => errors_in: u64, 6 => errors_out: u64, 7 => is_up: bool]); } _ => {} }
        Ok(i)
    }

    fn stor(&self, v: &Value) -> Result<StorageMetrics> {
        let mut s = StorageMetrics::default();
        match v { Value::Map(m) => { for (k, val) in m { match self.str(k).as_str() { "disks" => s.disks = self.disk_vec(val), "filesystems" => s.filesystems = self.fs_vec(val), _ => {} } } }
            Value::Array(a) => { if let Some(v) = a.first() { s.disks = self.disk_vec(v); } if let Some(v) = a.get(1) { s.filesystems = self.fs_vec(v); } } _ => {} }
        Ok(s)
    }

    fn disk(&self, v: &Value) -> Result<DiskMetrics> {
        let mut d = DiskMetrics { name: String::new(), read_bytes: 0, write_bytes: 0, read_ops: 0, write_ops: 0, utilization: 0.0 };
        match v { Value::Map(m) => { map_f!(self, m, d, { "name" => name: str, "read_bytes" => read_bytes: u64, "write_bytes" => write_bytes: u64, "read_ops" => read_ops: u64, "write_ops" => write_ops: u64, "utilization" => utilization: f64 }); }
            Value::Array(a) => { arr_f!(self, a, d, [0 => name: str, 1 => read_bytes: u64, 2 => write_bytes: u64, 3 => read_ops: u64, 4 => write_ops: u64, 5 => utilization: f64]); } _ => {} }
        Ok(d)
    }

    fn fs(&self, v: &Value) -> Result<FilesystemMetrics> {
        let mut f = FilesystemMetrics { mount_point: String::new(), total_space: 0, used_space: 0, available_space: 0, usage_percent: 0.0 };
        match v { Value::Map(m) => { map_f!(self, m, f, { "mount_point" => mount_point: str, "total_space" => total_space: u64, "used_space" => used_space: u64, "available_space" => available_space: u64, "usage_percent" => usage_percent: f64 }); }
            Value::Array(a) => { arr_f!(self, a, f, [0 => mount_point: str, 1 => total_space: u64, 2 => used_space: u64, 3 => available_space: u64, 4 => usage_percent: f64]); } _ => {} }
        Ok(f)
    }

    fn procs_sum(&self, v: &Value) -> Result<ProcessSummary> {
        let mut p = ProcessSummary::default();
        match v { Value::Map(m) => { for (k, val) in m { match self.str(k).as_str() { "total_processes" => p.total_processes = self.u32(val), "running_processes" => p.running_processes = self.u32(val), "sleeping_processes" => p.sleeping_processes = self.u32(val), "zombie_processes" => p.zombie_processes = self.u32(val), "total_threads" => p.total_threads = self.u32(val), "top_cpu_processes" => p.top_cpu_processes = self.proc_list(val), "top_memory_processes" => p.top_memory_processes = self.proc_list(val), "detected_services" => p.detected_services = self.svc_list(val), _ => {} } } }
            Value::Array(a) if a.len() >= 8 => { p.total_processes = self.u32(&a[0]); p.running_processes = self.u32(&a[1]); p.sleeping_processes = self.u32(&a[2]); p.zombie_processes = self.u32(&a[3]); p.total_threads = self.u32(&a[4]); p.top_cpu_processes = self.proc_list_arr(&a[5]); p.top_memory_processes = self.proc_list_arr(&a[6]); p.detected_services = self.svc_list_arr(&a[7]); } _ => {} }
        Ok(p)
    }

    fn proc_list(&self, v: &Value) -> Vec<ProcessMetrics> { let Value::Array(arr) = v else { return vec![] }; arr.iter().filter_map(|i| self.proc_map(i)).collect() }

    fn proc_map(&self, item: &Value) -> Option<ProcessMetrics> {
        let Value::Map(m) = item else { return None };
        let mut p = ProcessMetrics { pid: 0, name: String::new(), exe_path: None, cmd: vec![], cpu_usage: 0.0, memory_bytes: 0, memory_percent: 0.0, status: String::new(), user: None, start_time: None, threads: 0 };
        for (k, val) in m { match self.str(k).as_str() { "pid" => p.pid = self.u32(val), "name" => p.name = self.str(val), "exe_path" => p.exe_path = self.opt_str(val), "cmd" => p.cmd = self.str_vec(val), "cpu_usage" => p.cpu_usage = self.f64(val), "memory_bytes" => p.memory_bytes = self.u64(val), "memory_percent" => p.memory_percent = self.f64(val), "status" => p.status = self.str(val), "user" => p.user = self.opt_str(val), "start_time" => p.start_time = Some(self.i64(val)), "threads" => p.threads = self.u32(val), _ => {} } }
        (!p.name.is_empty()).then_some(p)
    }

    fn proc_list_arr(&self, v: &Value) -> Vec<ProcessMetrics> {
        let Value::Array(arr) = v else { return vec![] };
        arr.iter().filter_map(|item| { let Value::Array(a) = item else { return None }; if a.len() < 8 { return None } let nm = self.str(&a[1]); if nm.is_empty() { return None }
            Some(ProcessMetrics { pid: self.u32(&a[0]), name: nm, exe_path: self.opt_str(&a[2]), cmd: self.str_vec(&a[3]), cpu_usage: self.f64(&a[4]), memory_bytes: self.u64(&a[5]), memory_percent: self.f64(&a[6]), status: self.str(&a[7]), user: a.get(8).and_then(|x| self.opt_str(x)), start_time: a.get(9).map(|x| self.i64(x)), threads: a.get(10).map(|x| self.u32(x)).unwrap_or(0) })
        }).collect()
    }

    fn svc_list(&self, v: &Value) -> Vec<ServiceInfo> { let Value::Array(arr) = v else { return vec![] }; arr.iter().filter_map(|i| self.svc_map(i)).collect() }

    fn svc_map(&self, item: &Value) -> Option<ServiceInfo> {
        let Value::Map(m) = item else { return None };
        let mut s = ServiceInfo { name: String::new(), display_name: None, status: String::new(), process_count: 0, total_cpu: 0.0, total_memory: 0, pids: vec![], exe_path: None, command: None };
        for (k, val) in m { match self.str(k).as_str() { "name" => s.name = self.str(val), "display_name" => s.display_name = self.opt_str(val), "status" => s.status = self.str(val), "process_count" => s.process_count = self.u32(val), "total_cpu" => s.total_cpu = self.f64(val), "total_memory" => s.total_memory = self.u64(val), "pids" => s.pids = self.u32_vec(val), "exe_path" => s.exe_path = self.opt_str(val), "command" => s.command = self.opt_str(val), _ => {} } }
        (!s.name.is_empty()).then_some(s)
    }

    fn svc_list_arr(&self, v: &Value) -> Vec<ServiceInfo> {
        let Value::Array(arr) = v else { return vec![] };
        arr.iter().filter_map(|item| { let Value::Array(a) = item else { return None }; if a.len() < 6 { return None } let nm = self.str(&a[0]); if nm.is_empty() { return None }
            Some(ServiceInfo { name: nm, display_name: self.opt_str(&a[1]), status: self.str(&a[2]), process_count: self.u32(&a[3]), total_cpu: self.f64(&a[4]), total_memory: self.u64(&a[5]), pids: a.get(6).map(|x| self.u32_vec(x)).unwrap_or_default(), exe_path: a.get(7).and_then(|x| self.opt_str(x)), command: a.get(8).and_then(|x| self.opt_str(x)) })
        }).collect()
    }

    fn teleco(&self, v: &Value) -> Result<HashMap<String, HashMap<String, f64>>> {
        let Value::Map(outer) = v else { return Ok(HashMap::new()) };
        Ok(outer.iter().map(|(k, inner)| (self.str(k), match inner { Value::Map(m) => m.iter().map(|(ik, iv)| (self.str(ik), self.f64(iv))).collect(), _ => HashMap::new() })).collect())
    }

    #[inline] fn str(&self, v: &Value) -> String { match v { Value::String(s) => s.as_str().map(String::from).unwrap_or_default(), Value::Integer(i) => i.to_string(), _ => String::new() } }
    #[inline] fn opt_str(&self, v: &Value) -> Option<String> { let s = self.str(v); (!s.is_empty()).then_some(s) }
    #[inline] fn i64(&self, v: &Value) -> i64 { match v { Value::Integer(i) => i.as_i64().unwrap_or(0), Value::F32(f) => *f as i64, Value::F64(f) => *f as i64, _ => 0 } }
    #[inline] fn u64(&self, v: &Value) -> u64 { match v { Value::Integer(i) => i.as_u64().unwrap_or(0), Value::F32(f) => *f as u64, Value::F64(f) => *f as u64, _ => 0 } }
    #[inline] fn u32(&self, v: &Value) -> u32 { self.u64(v) as u32 }
    #[inline] fn f64(&self, v: &Value) -> f64 { match v { Value::F32(f) => *f as f64, Value::F64(f) => *f, Value::Integer(i) => i.as_f64().unwrap_or(0.0), _ => 0.0 } }
    #[inline] fn bool(&self, v: &Value) -> bool { match v { Value::Boolean(b) => *b, Value::Integer(i) => i.as_u64().unwrap_or(0) != 0, _ => false } }
    #[inline] fn arr(&self, v: &Value) -> Option<Vec<Value>> { match v { Value::Array(a) => Some(a.clone()), _ => None } }
    #[inline] fn f64_vec(&self, v: &Value) -> Vec<f64> { match v { Value::Array(a) => a.iter().map(|x| self.f64(x)).collect(), _ => vec![] } }
    #[inline] fn u32_vec(&self, v: &Value) -> Vec<u32> { match v { Value::Array(a) => a.iter().map(|x| self.u32(x)).collect(), _ => vec![] } }
    #[inline] fn str_vec(&self, v: &Value) -> Vec<String> { match v { Value::Array(a) => a.iter().map(|x| self.str(x)).filter(|s| !s.is_empty()).collect(), _ => vec![] } }
    #[inline] fn sensor_vec(&self, v: &Value) -> Vec<ThermalSensor> { match v { Value::Array(a) => a.iter().filter_map(|x| self.sensor(x).ok()).collect(), _ => vec![] } }
    #[inline] fn disk_vec(&self, v: &Value) -> Vec<DiskMetrics> { match v { Value::Array(a) => a.iter().filter_map(|x| self.disk(x).ok()).collect(), _ => vec![] } }
    #[inline] fn fs_vec(&self, v: &Value) -> Vec<FilesystemMetrics> { match v { Value::Array(a) => a.iter().filter_map(|x| self.fs(x).ok()).collect(), _ => vec![] } }
}

impl Default for SystemMetrics {
    fn default() -> Self { Self { node_id: String::new(), timestamp: 0, hardware: HardwareMetrics::default(), network: NetworkMetrics::default(), storage: StorageMetrics::default(), processes: None, source_type: None, device_type: None, connection_protocol: None, location: None, teleco_specific: None } }
}
