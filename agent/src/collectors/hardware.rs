use std::time::Duration;
use sysinfo::{Components, System};
use tracing::{debug, warn};

use crate::error::AgentError;
use crate::metrics::{HardwareMetrics, MemoryInfo, ThermalReading};
use crate::types::{Celsius, Ms, Percent};

const CPU_SAMPLE_DELAY_MS: Ms = 200;

pub struct HardwareCollector {
    sys: System,
    thermal: Components,
    has_thermal: bool,
}

impl HardwareCollector {
    pub fn new() -> Result<Self, AgentError> {
        let mut sys = System::new();
        sys.refresh_cpu_all();
        
        let thermal = Components::new_with_refreshed_list();
        let has_thermal = !thermal.is_empty();
        
        if !has_thermal {
            // TODO no todos los sitemas tienen por defecto sensores termicos, buscar otra forma de obtener la temperatura
            debug!("no se detectaron sensores de temperatura");
        } else {
            debug!("detectados {} sensores térmicos", thermal.len());
        }
        
        Ok(Self { sys, thermal, has_thermal })
    }

    pub async fn collect(&mut self) -> Result<HardwareMetrics, AgentError> {
        let cpu = self.sample_cpu().await;
        let memory = self.read_memory();
        let temps = self.read_temperatures();
        
        Ok(HardwareMetrics {
            cpu_usage: cpu,
            memory_usage: memory,
            thermal_sensors: temps,
        })
    }

    async fn sample_cpu(&mut self) -> Vec<Percent> {
        self.sys.refresh_cpu_all();
        tokio::time::sleep(Duration::from_millis(CPU_SAMPLE_DELAY_MS)).await;
        self.sys.refresh_cpu_all();
        
        self.sys.cpus()
            .iter()
            .map(|cpu| cpu.cpu_usage())
            .collect()
    }

    fn read_memory(&self) -> MemoryInfo {
        let mut s = System::new();
        s.refresh_memory();
        
        let total = s.total_memory();
        let used = s.used_memory();
        let available = s.available_memory();
        let free = s.free_memory();
        
        // TODO habrá que darle una vuelta en el futuro recuerd aproblema de exactitud
        let cached = available.saturating_sub(free);
        
        MemoryInfo {
            total,
            used,
            available,
            cached,
            buffers: 0,
        }
    }

    fn read_temperatures(&mut self) -> Vec<ThermalReading> {
        if !self.has_thermal {
            return Vec::new();
        }
        
        self.thermal.refresh(false);
        
        self.thermal.iter()
            .filter_map(|component| {
                let temp = component.temperature().filter(|&t| t > 0.0)?;
                let label = component.label().to_string();
                
                // control para mitigar temperatura absurda o mala lectura
                if temp > 150.0 {
                    warn!("sensor '{}' reporta {}°C, ignorando", label, temp);
                    return None;
                }
                
                Some(ThermalReading {
                    name: label,
                    temperature: temp,
                    critical_temp: component.critical().map(|c| c as Celsius),
                })
            })
            .collect()
    }
}
