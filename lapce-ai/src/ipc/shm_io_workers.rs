/// Dedicated I/O worker threads for SPSC ring processing
/// with CPU pinning for optimal cache locality and reduced context switching
/// 
/// Cross-platform support:
/// - Linux: sched_setaffinity
/// - Windows: SetThreadAffinityMask
/// - macOS: thread_policy_set (THREAD_AFFINITY_POLICY)

use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use crossbeam::channel::{Sender, Receiver, unbounded};
use anyhow::Result;

use crate::ipc::spsc_shm_ring::SpscRing;

/// I/O worker configuration
#[derive(Clone)]
pub struct WorkerConfig {
    /// CPU core to pin to (None = no pinning)
    pub core_id: Option<usize>,
    /// Worker thread name
    pub name: String,
    /// Spin iterations before yielding
    pub spin_iterations: u32,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            core_id: None,
            name: "shm-worker".to_string(),
            spin_iterations: 1000,
        }
    }
}

/// Message from worker to Tokio dispatcher
pub struct WorkerMessage {
    pub conn_id: u64,
    pub data: Vec<u8>,
}

/// Dedicated I/O worker for processing SPSC rings
pub struct ShmIoWorker {
    handle: Option<JoinHandle<()>>,
    worker_id: usize,
    shutdown_tx: Sender<()>,
}

impl ShmIoWorker {
    /// Spawn a new I/O worker
    pub fn spawn(
        worker_id: usize,
        ring: Arc<SpscRing>,
        to_dispatcher: Sender<WorkerMessage>,
        config: WorkerConfig,
    ) -> Result<Self> {
        let (shutdown_tx, shutdown_rx) = unbounded();
        
        let handle = thread::Builder::new()
            .name(format!("{}-{}", config.name, worker_id))
            .spawn(move || {
                // Pin to CPU core if requested
                if let Some(core_id) = config.core_id {
                    if let Err(e) = Self::pin_to_core(core_id) {
                        eprintln!("Failed to pin worker {} to core {}: {}", worker_id, core_id, e);
                    }
                }
                
                // Main processing loop
                Self::process_loop(
                    worker_id,
                    ring,
                    to_dispatcher,
                    shutdown_rx,
                    config.spin_iterations,
                );
            })?;
        
        Ok(Self {
            handle: Some(handle),
            worker_id,
            shutdown_tx,
        })
    }
    
    /// Main processing loop
    fn process_loop(
        worker_id: usize,
        ring: Arc<SpscRing>,
        to_dispatcher: Sender<WorkerMessage>,
        shutdown_rx: Receiver<()>,
        spin_iterations: u32,
    ) {
        let mut spin_count = 0;
        
        loop {
            // Check for shutdown signal
            if shutdown_rx.try_recv().is_ok() {
                break;
            }
            
            // Try to read messages
            let mut processed = 0;
            while let Some(data) = ring.try_read() {
                let msg = WorkerMessage {
                    conn_id: worker_id as u64,
                    data,
                };
                
                // Send to Tokio dispatcher
                if to_dispatcher.send(msg).is_err() {
                    // Dispatcher closed, shutdown
                    return;
                }
                
                processed += 1;
                spin_count = 0; // Reset spin on successful read
            }
            
            if processed == 0 {
                // No messages available
                spin_count += 1;
                
                if spin_count < spin_iterations {
                    // Bounded spin
                    std::hint::spin_loop();
                } else {
                    // Yield to OS scheduler
                    thread::yield_now();
                    spin_count = 0;
                }
            }
        }
    }
    
    /// Pin current thread to specified CPU core
    #[cfg(target_os = "linux")]
    fn pin_to_core(core_id: usize) -> Result<()> {
        use libc::{cpu_set_t, CPU_SET, CPU_ZERO, sched_setaffinity, pid_t};
        
        unsafe {
            let mut cpu_set: cpu_set_t = std::mem::zeroed();
            CPU_ZERO(&mut cpu_set);
            CPU_SET(core_id, &mut cpu_set);
            
            let result = sched_setaffinity(
                0 as pid_t, // 0 = current thread
                std::mem::size_of::<cpu_set_t>(),
                &cpu_set,
            );
            
            if result != 0 {
                anyhow::bail!("sched_setaffinity failed: {}", std::io::Error::last_os_error());
            }
        }
        
        Ok(())
    }
    
    #[cfg(target_os = "windows")]
    fn pin_to_core(core_id: usize) -> Result<()> {
        use windows_sys::Win32::System::Threading::{
            SetThreadAffinityMask, GetCurrentThread,
        };
        
        unsafe {
            let mask: usize = 1 << core_id;
            let result = SetThreadAffinityMask(GetCurrentThread(), mask);
            
            if result == 0 {
                anyhow::bail!("SetThreadAffinityMask failed");
            }
        }
        
        Ok(())
    }
    
    #[cfg(target_os = "macos")]
    fn pin_to_core(core_id: usize) -> Result<()> {
        // macOS doesn't have direct CPU pinning like Linux
        // Use thread affinity policy as best effort
        use libc::{pthread_self, mach_port_t};
        
        #[repr(C)]
        struct thread_affinity_policy {
            affinity_tag: i32,
        }
        
        const THREAD_AFFINITY_POLICY: i32 = 4;
        
        extern "C" {
            fn pthread_mach_thread_np(thread: libc::pthread_t) -> mach_port_t;
            fn thread_policy_set(
                thread: mach_port_t,
                flavor: i32,
                policy_info: *const i32,
                count: u32,
            ) -> i32;
        }
        
        unsafe {
            let thread = pthread_mach_thread_np(pthread_self());
            let policy = thread_affinity_policy {
                affinity_tag: core_id as i32,
            };
            
            let result = thread_policy_set(
                thread,
                THREAD_AFFINITY_POLICY,
                &policy.affinity_tag as *const i32,
                1,
            );
            
            if result != 0 {
                anyhow::bail!("thread_policy_set failed: {}", result);
            }
        }
        
        Ok(())
    }
    
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    fn pin_to_core(_core_id: usize) -> Result<()> {
        // No-op for unsupported platforms
        Ok(())
    }
    
    /// Shutdown the worker gracefully
    pub fn shutdown(mut self) -> Result<()> {
        // Send shutdown signal
        let _ = self.shutdown_tx.send(());
        
        // Wait for thread to finish
        if let Some(handle) = self.handle.take() {
            handle.join()
                .map_err(|_| anyhow::anyhow!("Worker thread panicked"))?;
        }
        
        Ok(())
    }
}

impl Drop for ShmIoWorker {
    fn drop(&mut self) {
        // Best-effort shutdown
        let _ = self.shutdown_tx.send(());
        
        if let Some(handle) = self.handle.take() {
            // Give thread 100ms to finish, then detach
            let _ = std::thread::sleep(Duration::from_millis(100));
            let _ = handle.join();
        }
    }
}

/// Pool of I/O workers
pub struct ShmWorkerPool {
    workers: Vec<ShmIoWorker>,
    dispatcher_rx: Receiver<WorkerMessage>,
}

impl ShmWorkerPool {
    /// Create a new worker pool
    /// 
    /// # Arguments
    /// * `num_workers` - Number of worker threads (recommend: num_cores / 4)
    /// * `rings` - SPSC rings to process (one per worker)
    /// * `pin_cores` - Whether to pin workers to CPU cores
    pub fn new(
        num_workers: usize,
        rings: Vec<Arc<SpscRing>>,
        pin_cores: bool,
    ) -> Result<Self> {
        if rings.len() != num_workers {
            anyhow::bail!("Number of rings must match number of workers");
        }
        
        let (tx, rx) = unbounded();
        let mut workers = Vec::with_capacity(num_workers);
        
        for (worker_id, ring) in rings.into_iter().enumerate() {
            let config = WorkerConfig {
                core_id: if pin_cores { Some(worker_id) } else { None },
                name: format!("shm-worker-{}", worker_id),
                spin_iterations: 1000,
            };
            
            let worker = ShmIoWorker::spawn(worker_id, ring, tx.clone(), config)?;
            workers.push(worker);
        }
        
        Ok(Self {
            workers,
            dispatcher_rx: rx,
        })
    }
    
    /// Receive next message from any worker
    pub fn recv(&self) -> Result<WorkerMessage> {
        self.dispatcher_rx.recv()
            .map_err(|e| anyhow::anyhow!("Worker channel error: {}", e))
    }
    
    /// Receive next message async (for Tokio integration)
    pub async fn recv_async(&self) -> Result<WorkerMessage> {
        let rx = self.dispatcher_rx.clone();
        tokio::task::spawn_blocking(move || {
            rx.recv()
                .map_err(|e| anyhow::anyhow!("Worker channel error: {}", e))
        })
        .await
        .map_err(|e| anyhow::anyhow!("Join error: {}", e))?
    }
    
    /// Shutdown all workers
    pub fn shutdown(self) -> Result<()> {
        for worker in self.workers {
            worker.shutdown()?;
        }
        Ok(())
    }
    
    /// Get number of active workers
    pub fn num_workers(&self) -> usize {
        self.workers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::{alloc_zeroed, dealloc, Layout};
    use crate::ipc::spsc_shm_ring::RingHeader;
    
    #[test]
    fn test_worker_spawn_and_shutdown() {
        unsafe {
            let capacity = 64 * 1024;
            let header_layout = Layout::new::<RingHeader>();
            let data_layout = Layout::from_size_align(capacity, 64).unwrap();
            
            let header = alloc_zeroed(header_layout) as *mut RingHeader;
            let data = alloc_zeroed(data_layout);
            
            let ring = Arc::new(SpscRing::from_raw(header, data, capacity));
            let (tx, rx) = unbounded();
            
            let config = WorkerConfig {
                core_id: None, // No pinning for test
                ..Default::default()
            };
            
            let worker = ShmIoWorker::spawn(0, ring.clone(), tx, config).unwrap();
            
            // Write some data
            ring.try_write(b"test message");
            
            // Should receive it
            let msg = rx.recv_timeout(Duration::from_secs(1)).unwrap();
            assert_eq!(msg.data, b"test message");
            
            // Shutdown
            worker.shutdown().unwrap();
            
            dealloc(header as *mut u8, header_layout);
            dealloc(data, data_layout);
        }
    }
    
    #[test]
    fn test_worker_pool() {
        unsafe {
            let num_workers = 2;
            let capacity = 64 * 1024;
            
            let mut rings = Vec::new();
            for _ in 0..num_workers {
                let header_layout = Layout::new::<RingHeader>();
                let data_layout = Layout::from_size_align(capacity, 64).unwrap();
                
                let header = alloc_zeroed(header_layout) as *mut RingHeader;
                let data = alloc_zeroed(data_layout);
                
                rings.push(Arc::new(SpscRing::from_raw(header, data, capacity)));
            }
            
            let rings_clone = rings.clone();
            let pool = ShmWorkerPool::new(num_workers, rings_clone, false).unwrap();
            
            // Write to each ring
            rings[0].try_write(b"from worker 0");
            rings[1].try_write(b"from worker 1");
            
            // Should receive both
            let mut received = Vec::new();
            for _ in 0..2 {
                let msg = pool.recv().unwrap();
                received.push(msg.data);
            }
            
            assert!(received.contains(&b"from worker 0".to_vec()));
            assert!(received.contains(&b"from worker 1".to_vec()));
            
            pool.shutdown().unwrap();
        }
    }
}
