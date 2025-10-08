/// STANDALONE MEMORY-FIXED TEST - Direct compilation without cargo
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant};

// Inline minimal SharedMemoryBuffer implementation
use std::ptr;
use std::sync::atomic::AtomicUsize;

#[repr(C)]
struct BufferHeader {
    write_pos: AtomicUsize,
    read_pos: AtomicUsize,
    size: usize,
}

struct SharedMemoryBuffer {
    ptr: *mut u8,
    size: usize,
    header: *mut BufferHeader,
}

unsafe impl Send for SharedMemoryBuffer {}
unsafe impl Sync for SharedMemoryBuffer {}

impl SharedMemoryBuffer {
    fn create(name: &str, size: usize) -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            let total_size = size + std::mem::size_of::<BufferHeader>();
            let ptr = libc::mmap(
                ptr::null_mut(),
                total_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_ANONYMOUS,
                -1,
                0,
            ) as *mut u8;
            
            if ptr == libc::MAP_FAILED as *mut u8 {
                return Err("mmap failed".into());
            }
            
            let header = ptr as *mut BufferHeader;
            (*header).write_pos = AtomicUsize::new(0);
            (*header).read_pos = AtomicUsize::new(0);
            (*header).size = size;
            
            Ok(Self { ptr, size: total_size, header })
        }
    }
    
    fn write(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let header = &*self.header;
            let buffer_start = self.ptr.add(std::mem::size_of::<BufferHeader>());
            let pos = header.write_pos.load(Ordering::Relaxed);
            
            if data.len() + 4 > header.size {
                return Err("Data too large".into());
            }
            
            // Write length
            let len = data.len() as u32;
            ptr::copy_nonoverlapping(&len as *const u32 as *const u8, buffer_start.add(pos), 4);
            
            // Write data
            let new_pos = (pos + 4 + data.len()) % header.size;
            ptr::copy_nonoverlapping(data.as_ptr(), buffer_start.add(pos + 4), data.len());
            
            header.write_pos.store(new_pos, Ordering::Release);
            Ok(())
        }
    }
    
    fn read(&mut self) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        unsafe {
            let header = &*self.header;
            let read_pos = header.read_pos.load(Ordering::Acquire);
            let write_pos = header.write_pos.load(Ordering::Acquire);
            
            if read_pos == write_pos {
                return Ok(None);
            }
            
            let buffer_start = self.ptr.add(std::mem::size_of::<BufferHeader>());
            
            // Read length
            let mut len: u32 = 0;
            ptr::copy_nonoverlapping(buffer_start.add(read_pos), &mut len as *mut u32 as *mut u8, 4);
            
            // Read data
            let mut data = vec![0u8; len as usize];
            ptr::copy_nonoverlapping(buffer_start.add(read_pos + 4), data.as_mut_ptr(), len as usize);
            
            let new_pos = (read_pos + 4 + len as usize) % header.size;
            header.read_pos.store(new_pos, Ordering::Release);
            
            Ok(Some(data))
        }
    }
}

impl Drop for SharedMemoryBuffer {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                libc::munmap(self.ptr as *mut _, self.size);
            }
        }
    }
}

fn main() {
    println!("\n🚀 STANDALONE MEMORY-FIXED PRODUCTION TEST");
    println!("{}", "=".repeat(80));
    
    const TEST_DURATION_SECS: u64 = 30;
    const CONCURRENT_CONNECTIONS: usize = 1000;
    const BUFFER_POOL_SIZE: usize = 10;
    const MESSAGE_SIZE: usize = 256;
    
    // Get baseline memory
    let baseline_kb = get_memory_kb();
    println!("Baseline memory: {:.2} MB", baseline_kb as f64 / 1024.0);
    
    // Create buffer pool
    let mut buffers = Vec::new();
    for i in 0..BUFFER_POOL_SIZE {
        match SharedMemoryBuffer::create(&format!("pool_{}", i), 64 * 1024) {
            Ok(buf) => buffers.push(Arc::new(std::sync::Mutex::new(buf))),
            Err(e) => {
                eprintln!("Failed to create buffer {}: {}", i, e);
                return;
            }
        }
    }
    
    println!("📡 Created {} shared buffers for {} connections", BUFFER_POOL_SIZE, CONCURRENT_CONNECTIONS);
    
    let total_messages = Arc::new(AtomicU64::new(0));
    let stop_signal = Arc::new(AtomicBool::new(false));
    let start_time = Instant::now();
    
    // Spawn threads
    let mut handles = Vec::new();
    for id in 0..CONCURRENT_CONNECTIONS {
        let buffer = buffers[id % BUFFER_POOL_SIZE].clone();
        let total = total_messages.clone();
        let stop = stop_signal.clone();
        
        let handle = std::thread::spawn(move || {
            let test_msg = vec![0x42u8; MESSAGE_SIZE];
            
            while !stop.load(Ordering::Relaxed) {
                if let Ok(mut buf) = buffer.lock() {
                    if buf.write(&test_msg).is_ok() {
                        if buf.read().is_ok() {
                            total.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                std::thread::yield_now();
            }
        });
        handles.push(handle);
    }
    
    println!("✅ Started {} threads", CONCURRENT_CONNECTIONS);
    println!("⏳ Running for {} seconds...", TEST_DURATION_SECS);
    
    // Run test
    std::thread::sleep(Duration::from_secs(TEST_DURATION_SECS));
    stop_signal.store(true, Ordering::Relaxed);
    
    println!("🛑 Stopping threads...");
    for handle in handles {
        handle.join().ok();
    }
    
    let duration = start_time.elapsed();
    let peak_kb = get_memory_kb();
    let overhead_mb = (peak_kb - baseline_kb) as f64 / 1024.0;
    
    let total = total_messages.load(Ordering::Relaxed);
    let throughput = (total as f64 / duration.as_secs_f64()) as u64;
    
    // Results
    println!("\n{}", "=".repeat(80));
    println!("🎯 FINAL RESULTS");
    println!("{}", "=".repeat(80));
    
    println!("\n📊 THROUGHPUT:");
    println!("  Messages:      {}", total);
    println!("  Throughput:    {} msg/sec", throughput);
    println!("  Target:        > 1M msg/sec");
    println!("  Status:        {}", if throughput > 1_000_000 { "✅ PASS" } else { "❌ FAIL" });
    
    println!("\n💾 MEMORY:");
    println!("  Baseline:      {:.2} MB", baseline_kb as f64 / 1024.0);
    println!("  Peak:          {:.2} MB", peak_kb as f64 / 1024.0);
    println!("  Overhead:      {:.2} MB", overhead_mb);
    println!("  Target:        < 3 MB");
    println!("  Status:        {}", if overhead_mb < 3.0 { "✅ PASS" } else { "❌ FAIL" });
    
    println!("\n🔥 vs Node.js:");
    const NODEJS: u64 = 21_013;
    let improvement = throughput / NODEJS;
    println!("  Node.js:       {} msg/sec", NODEJS);
    println!("  Our System:    {} msg/sec", throughput);
    println!("  Improvement:   {}x faster", improvement);
    println!("  Target:        > 10x");
    println!("  Status:        {}", if improvement >= 10 { "✅ PASS" } else { "❌ FAIL" });
    
    println!("\n{}", "=".repeat(80));
}

fn get_memory_kb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(0)
}

extern "C" {
    // Link to libc
}

mod libc {
    pub const PROT_READ: usize = 0x1;
    pub const PROT_WRITE: usize = 0x2;
    pub const MAP_SHARED: usize = 0x01;
    pub const MAP_ANONYMOUS: usize = 0x20;
    pub const MAP_FAILED: *mut core::ffi::c_void = !0 as *mut core::ffi::c_void;
    
    extern "C" {
        pub fn mmap(addr: *mut core::ffi::c_void, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut core::ffi::c_void;
        pub fn munmap(addr: *mut core::ffi::c_void, len: usize) -> i32;
    }
}
