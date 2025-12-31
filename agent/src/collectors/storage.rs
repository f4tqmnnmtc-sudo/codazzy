use sysinfo::Disks;
use tracing::debug;

use crate::error::AgentError;
use crate::metrics::{DiskStats, FilesystemStats, StorageMetrics};
use crate::types::Percent;

pub struct StorageCollector;

impl StorageCollector {
    pub fn new() -> Result<Self, AgentError> {
        Ok(Self)
    }

    pub async fn collect(&mut self) -> Result<StorageMetrics, AgentError> {
        let disks = Disks::new_with_refreshed_list();
        
        let count = disks.list().len();
        let mut disk_stats = Vec::with_capacity(count);
        let mut fs_stats = Vec::with_capacity(count);

        for disk in disks.list() {
            let name = disk.name().to_string_lossy();
            let mount = disk.mount_point().to_string_lossy();
            
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total.saturating_sub(available);
            
            let usage_pct = if total > 0 {
                (used as f64 / total as f64 * 100.0) as Percent
            } else {
                0.0
            };
            
            let io = disk.usage();
            
            debug!(
                "disco {}: {:.1}% usado ({} de {} bytes), mount={}",
                name, usage_pct, used, total, mount
            );
            
            disk_stats.push(DiskStats {
                name: name.into_owned(),
                read_bytes: io.total_read_bytes,
                write_bytes: io.total_written_bytes,
                read_ops: io.read_bytes / 4096,
                write_ops: io.written_bytes / 4096,
                utilization: usage_pct,
            });
            
            fs_stats.push(FilesystemStats {
                mount_point: mount.into_owned(),
                total_space: total,
                used_space: used,
                available_space: available,
                usage_percent: usage_pct,
            });
        }
        
        Ok(StorageMetrics {
            disks: disk_stats,
            filesystems: fs_stats,
        })
    }
}

impl Default for StorageCollector {
    fn default() -> Self {
        Self
    }
}
