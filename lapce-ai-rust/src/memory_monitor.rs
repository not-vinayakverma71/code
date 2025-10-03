// PHASE 2.1: MEMORY MONITOR - Track memory usage
use std::time::Instant;
use std::collections::HashMap;
use sysinfo::{System, Pid};

pub struct MemoryMonitor {
    baseline: u64,
    peak: u64,
    samples: Vec<(Instant, u64)>,
    components: HashMap<String, u64>,
}

impl MemoryMonitor {
    pub fn new() -> Self {
        let baseline = Self::get_current_memory();
        
        Self {
            baseline,
            peak: baseline,
            samples: Vec::new(),
            components: HashMap::new(),
        }
    }
    
    fn get_current_memory() -> u64 {
        let mut system = System::new_all();
        system.refresh_all();
        
        let pid = std::process::id();
        if let Some(process) = system.process(Pid::from_u32(pid)) {
            process.memory() * 1024 // Convert KB to bytes
        } else {
            0
        }
    }
    
    pub fn sample(&mut self, label: &str) {
        let current = Self::get_current_memory();
        let now = Instant::now();
        
        self.samples.push((now, current));
        self.peak = self.peak.max(current);
        
        if !label.is_empty() {
            let delta = current.saturating_sub(self.baseline);
            self.components.insert(label.to_string(), delta);
        }
    }
    
    pub fn report(&self) -> MemoryReport {
        let current = Self::get_current_memory();
        let used = current.saturating_sub(self.baseline);
        let peak_usage = self.peak.saturating_sub(self.baseline);
        
        MemoryReport {
            baseline_bytes: self.baseline,
            current_bytes: current,
            used_bytes: used,
            peak_bytes: peak_usage,
            used_mb: (used as f64) / 1_048_576.0,
            peak_mb: (peak_usage as f64) / 1_048_576.0,
            components: self.components.clone(),
        }
    }
}

#[derive(Debug)]
pub struct MemoryReport {
    pub baseline_bytes: u64,
    pub current_bytes: u64,
    pub used_bytes: u64,
    pub peak_bytes: u64,
    pub used_mb: f64,
    pub peak_mb: f64,
    pub components: HashMap<String, u64>,
}

impl MemoryReport {
    pub fn print_summary(&self) {
        println!("\n📊 MEMORY USAGE REPORT");
        println!("{}", "=".repeat(40));
        println!("Baseline: {:.2} MB", self.baseline_bytes as f64 / 1_048_576.0);
        println!("Current:  {:.2} MB", self.current_bytes as f64 / 1_048_576.0);
        println!("Used:     {:.2} MB", self.used_mb);
        println!("Peak:     {:.2} MB", self.peak_mb);
        
        if !self.components.is_empty() {
            println!("\nComponent Breakdown:");
            for (component, bytes) in &self.components {
                let mb = (*bytes as f64) / 1_048_576.0;
                println!("  • {}: {:.2} MB", component, mb);
            }
        }
        
        // Check if we meet the <10MB target
        if self.used_mb < 10.0 {
            println!("\n✅ MEMORY TARGET MET: {:.2} MB < 10 MB", self.used_mb);
        } else {
            println!("\n⚠️  MEMORY TARGET EXCEEDED: {:.2} MB > 10 MB", self.used_mb);
        }
    }
}
