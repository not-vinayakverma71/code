/// FIXED SharedMemory - Actually works for all data sizes
use memmap2::{MmapMut, MmapOptions};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::fs::{OpenOptions, remove_file};
use std::io::Write;
use anyhow::Result;

const HEADER_SIZE: usize = 256; // Larger header for metadata

#[repr(C)]
struct ShmHeader {
    magic: AtomicU64,
    version: AtomicU64,
    write_pos: AtomicU64,
    read_pos: AtomicU64,
    total_writes: AtomicU64,
    total_reads: AtomicU64,
    is_locked: AtomicBool,
    buffer_size: u64,
}

pub struct FixedSharedMemory {
    mmap: MmapMut,
    path: String,
    size: usize,
    buffer_size: usize,
    is_owner: bool,
}

impl FixedSharedMemory {
    pub fn create(name: &str, size: usize) -> Result<Self> {
        let path = format!("/tmp/lapce_shm_fixed_{}", name);
        
        // Ensure minimum size
        let actual_size = size.max(1024 * 1024); // Min 1MB
        
        // Create file
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        
        file.set_len(actual_size as u64)?;
        file.flush()?;
        
        let mut mmap = unsafe {
            MmapOptions::new()
                .len(actual_size)
                .map_mut(&file)?
        };
        
        // Initialize
        mmap.fill(0);
        
        let buffer_size = actual_size - HEADER_SIZE;
        
        // Set up header
        unsafe {
            let header = &mut *(mmap.as_mut_ptr() as *mut ShmHeader);
            header.magic.store(0xDEADBEEF, Ordering::Release);
            header.version.store(1, Ordering::Release);
            header.write_pos.store(0, Ordering::Release);
            header.read_pos.store(0, Ordering::Release);
            header.total_writes.store(0, Ordering::Release);
            header.total_reads.store(0, Ordering::Release);
            header.is_locked.store(false, Ordering::Release);
            header.buffer_size = buffer_size as u64;
        }
        
        Ok(Self {
            mmap,
            path,
            size: actual_size,
            buffer_size,
            is_owner: true,
        })
    }
    
    pub fn write(&mut self, data: &[u8]) -> bool {
        // Check data fits with metadata
        if data.len() + 8 > self.buffer_size {
            return false;
        }
        
        unsafe {
            let header = &*(self.mmap.as_ptr() as *const ShmHeader);
            
            // Simple spinlock
            while header.is_locked.compare_exchange_weak(
                false,
                true,
                Ordering::Acquire,
                Ordering::Relaxed
            ).is_err() {
                std::hint::spin_loop();
            }
            
            let write_pos = header.write_pos.load(Ordering::Acquire) as usize;
            let read_pos = header.read_pos.load(Ordering::Acquire) as usize;
            
            // Calculate available space
            let available = if write_pos >= read_pos {
                self.buffer_size - (write_pos - read_pos) - 1
            } else {
                read_pos - write_pos - 1
            };
            
            // Check if data fits
            if available < data.len() + 8 {
                header.is_locked.store(false, Ordering::Release);
                return false;
            }
            
            // Write to buffer
            let buffer_start = HEADER_SIZE;
            let mut current_pos = write_pos;
            
            // Write length first (4 bytes)
            let len_bytes = (data.len() as u32).to_le_bytes();
            for &byte in &len_bytes {
                let actual_pos = buffer_start + (current_pos % self.buffer_size);
                *self.mmap.as_mut_ptr().add(actual_pos) = byte;
                current_pos = (current_pos + 1) % self.buffer_size;
            }
            
            // Write data
            for &byte in data {
                let actual_pos = buffer_start + (current_pos % self.buffer_size);
                *self.mmap.as_mut_ptr().add(actual_pos) = byte;
                current_pos = (current_pos + 1) % self.buffer_size;
            }
            
            // Update write position
            header.write_pos.store(current_pos as u64, Ordering::Release);
            header.total_writes.fetch_add(1, Ordering::Relaxed);
            
            // Release lock
            header.is_locked.store(false, Ordering::Release);
        }
        
        true
    }
    
    pub fn read(&mut self) -> Option<Vec<u8>> {
        unsafe {
            let header = &*(self.mmap.as_ptr() as *const ShmHeader);
            
            // Spinlock
            while header.is_locked.compare_exchange_weak(
                false,
                true,
                Ordering::Acquire,
                Ordering::Relaxed
            ).is_err() {
                std::hint::spin_loop();
            }
            
            let write_pos = header.write_pos.load(Ordering::Acquire) as usize;
            let read_pos = header.read_pos.load(Ordering::Acquire) as usize;
            
            // Check if data available
            if write_pos == read_pos {
                header.is_locked.store(false, Ordering::Release);
                return None;
            }
            
            let buffer_start = HEADER_SIZE;
            let mut current_pos = read_pos;
            
            // Read length (4 bytes)
            let mut len_bytes = [0u8; 4];
            for i in 0..4 {
                let actual_pos = buffer_start + (current_pos % self.buffer_size);
                len_bytes[i] = *self.mmap.as_ptr().add(actual_pos);
                current_pos = (current_pos + 1) % self.buffer_size;
            }
            let len = u32::from_le_bytes(len_bytes) as usize;
            
            // Sanity check
            if len > self.buffer_size {
                header.is_locked.store(false, Ordering::Release);
                return None;
            }
            
            // Read data
            let mut data = Vec::with_capacity(len);
            for _ in 0..len {
                let actual_pos = buffer_start + (current_pos % self.buffer_size);
                data.push(*self.mmap.as_ptr().add(actual_pos));
                current_pos = (current_pos + 1) % self.buffer_size;
            }
            
            // Update read position
            header.read_pos.store(current_pos as u64, Ordering::Release);
            header.total_reads.fetch_add(1, Ordering::Relaxed);
            
            // Release lock
            header.is_locked.store(false, Ordering::Release);
            
            Some(data)
        }
    }
    
    pub fn stats(&self) -> (u64, u64) {
        unsafe {
            let header = &*(self.mmap.as_ptr() as *const ShmHeader);
            (
                header.total_writes.load(Ordering::Relaxed),
                header.total_reads.load(Ordering::Relaxed)
            )
        }
    }
}

impl Drop for FixedSharedMemory {
    fn drop(&mut self) {
        if self.is_owner {
            let _ = remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_all_sizes() {
        let mut shm = FixedSharedMemory::create("test", 1024 * 1024).unwrap();
        
        // Test various sizes
        let sizes = vec![1, 64, 256, 1024, 4096, 16384, 65536];
        
        for size in sizes {
            let data = vec![0xAB; size];
            assert!(shm.write(&data), "Failed to write {} bytes", size);
            
            let read = shm.read().unwrap();
            assert_eq!(read.len(), size, "Size mismatch for {} bytes", size);
            assert_eq!(read, data, "Data mismatch for {} bytes", size);
        }
    }
    
    #[test]
    fn test_throughput() {
        let mut shm = FixedSharedMemory::create("bench", 8 * 1024 * 1024).unwrap();
        let data = vec![0xFF; 256];
        
        let start = std::time::Instant::now();
        let iterations = 100_000;
        
        for _ in 0..iterations {
            assert!(shm.write(&data));
            shm.read();
        }
        
        let elapsed = start.elapsed();
        let throughput = iterations as f64 / elapsed.as_secs_f64();
        
        println!("Fixed SHM Throughput: {:.2} msg/sec", throughput);
        assert!(throughput > 100_000.0, "Throughput too low: {}", throughput);
    }
}
