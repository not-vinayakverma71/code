/// Day 2: Optimized SharedMemory for 1M+ msg/sec
use memmap2::{MmapMut, MmapOptions};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::fs::OpenOptions;
use std::io::Write;
use anyhow::Result;

const CACHE_LINE_SIZE: usize = 64;
const HEADER_SIZE: usize = 256;

#[repr(C, align(64))] // Cache-line aligned
struct ShmHeader {
    // Separate cache lines to avoid false sharing
    write_pos: AtomicU64,
    _pad1: [u8; CACHE_LINE_SIZE - 8],
    
    read_pos: AtomicU64,  
    _pad2: [u8; CACHE_LINE_SIZE - 8],
    
    is_locked: AtomicBool,
    _pad3: [u8; CACHE_LINE_SIZE - 1],
    
    stats: Statistics,
}

#[repr(C)]
struct Statistics {
    total_writes: AtomicU64,
    total_reads: AtomicU64,
    buffer_size: u64,
}

pub struct OptimizedSharedMemory {
    mmap: MmapMut,
    size: usize,
    buffer_size: usize,
}

impl OptimizedSharedMemory {
    pub fn create(name: &str, size: usize) -> Result<Self> {
        let path = format!("/tmp/lapce_shm_opt_{}", name);
        let actual_size = size.max(1024 * 1024);
        
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
        
        mmap.fill(0);
        
        let buffer_size = actual_size - HEADER_SIZE;
        
        unsafe {
            let header = &mut *(mmap.as_mut_ptr() as *mut ShmHeader);
            header.write_pos.store(0, Ordering::Relaxed);
            header.read_pos.store(0, Ordering::Relaxed);
            header.is_locked.store(false, Ordering::Relaxed);
            header.stats.buffer_size = buffer_size as u64;
        }
        
        Ok(Self {
            mmap,
            size: actual_size,
            buffer_size,
        })
    }
    
    pub fn open_existing(name: &str, size: usize) -> Result<Self> {
        let path = format!("/tmp/lapce_shm_opt_{}", name);
        let actual_size = size.max(1024 * 1024);
        
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)?;
        
        let mmap = unsafe {
            MmapOptions::new()
                .len(actual_size)
                .map_mut(&file)?
        };
        
        let buffer_size = actual_size - HEADER_SIZE;
        
        Ok(Self {
            mmap,
            size: actual_size,
            buffer_size,
        })
    }
    
    #[inline(always)]
    pub fn write(&mut self, data: &[u8]) -> bool {
        let data_len = data.len();
        if data_len + 4 > self.buffer_size {
            return false;
        }
        
        unsafe {
            let header = &*(self.mmap.as_ptr() as *const ShmHeader);
            
            // Try-lock with backoff
            let mut spin_count = 0;
            while header.is_locked.compare_exchange_weak(
                false,
                true,
                Ordering::Acquire,
                Ordering::Relaxed
            ).is_err() {
                if spin_count < 10 {
                    std::hint::spin_loop();
                    spin_count += 1;
                } else {
                    std::thread::yield_now();
                    spin_count = 0;
                }
            }
            
            // Fast path - no modulo operations
            let write_pos = header.write_pos.load(Ordering::Relaxed) as usize;
            let read_pos = header.read_pos.load(Ordering::Relaxed) as usize;
            
            let available = if write_pos >= read_pos {
                self.buffer_size - (write_pos - read_pos) - 1
            } else {
                read_pos - write_pos - 1
            };
            
            if available < data_len + 4 {
                header.is_locked.store(false, Ordering::Release);
                return false;
            }
            
            // Optimized write with minimal branches
            let buffer_ptr = self.mmap.as_mut_ptr().add(HEADER_SIZE);
            let mut pos = write_pos;
            
            // Write length (4 bytes)
            let len_bytes = (data_len as u32).to_le_bytes();
            for &byte in &len_bytes {
                if pos >= self.buffer_size {
                    pos = 0;
                }
                *buffer_ptr.add(pos) = byte;
                pos += 1;
            }
            
            // Write data - optimized with memcpy when possible
            if pos + data_len <= self.buffer_size {
                // Fast path - single memcpy
                std::ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    buffer_ptr.add(pos),
                    data_len
                );
                pos += data_len;
            } else {
                // Wrap around
                let first_chunk = self.buffer_size - pos;
                std::ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    buffer_ptr.add(pos),
                    first_chunk
                );
                std::ptr::copy_nonoverlapping(
                    data.as_ptr().add(first_chunk),
                    buffer_ptr,
                    data_len - first_chunk
                );
                pos = data_len - first_chunk;
            }
            
            if pos >= self.buffer_size {
                pos -= self.buffer_size;
            }
            
            header.write_pos.store(pos as u64, Ordering::Release);
            header.stats.total_writes.fetch_add(1, Ordering::Relaxed);
            header.is_locked.store(false, Ordering::Release);
        }
        
        true
    }
    
    #[inline(always)]
    pub fn read(&mut self) -> Option<Vec<u8>> {
        unsafe {
            let header = &*(self.mmap.as_ptr() as *const ShmHeader);
            
            // Fast try-lock
            if !header.is_locked.compare_exchange(
                false,
                true,
                Ordering::Acquire,
                Ordering::Relaxed
            ).is_ok() {
                return None;
            }
            
            let write_pos = header.write_pos.load(Ordering::Acquire) as usize;
            let read_pos = header.read_pos.load(Ordering::Relaxed) as usize;
            
            if write_pos == read_pos {
                header.is_locked.store(false, Ordering::Release);
                return None;
            }
            
            let buffer_ptr = self.mmap.as_ptr().add(HEADER_SIZE);
            let mut pos = read_pos;
            
            // Read length
            let mut len_bytes = [0u8; 4];
            for i in 0..4 {
                if pos >= self.buffer_size {
                    pos = 0;
                }
                len_bytes[i] = *buffer_ptr.add(pos);
                pos += 1;
            }
            let data_len = u32::from_le_bytes(len_bytes) as usize;
            
            if data_len > self.buffer_size {
                header.is_locked.store(false, Ordering::Release);
                return None;
            }
            
            // Read data - optimized
            let mut data = Vec::with_capacity(data_len);
            data.set_len(data_len);
            
            if pos + data_len <= self.buffer_size {
                // Fast path
                std::ptr::copy_nonoverlapping(
                    buffer_ptr.add(pos),
                    data.as_mut_ptr(),
                    data_len
                );
                pos += data_len;
            } else {
                // Wrap around
                let first_chunk = self.buffer_size - pos;
                std::ptr::copy_nonoverlapping(
                    buffer_ptr.add(pos),
                    data.as_mut_ptr(),
                    first_chunk
                );
                std::ptr::copy_nonoverlapping(
                    buffer_ptr,
                    data.as_mut_ptr().add(first_chunk),
                    data_len - first_chunk
                );
                pos = data_len - first_chunk;
            }
            
            if pos >= self.buffer_size {
                pos -= self.buffer_size;
            }
            
            header.read_pos.store(pos as u64, Ordering::Release);
            header.stats.total_reads.fetch_add(1, Ordering::Relaxed);
            header.is_locked.store(false, Ordering::Release);
            
            Some(data)
        }
    }
    
    pub fn stats(&self) -> (u64, u64) {
        unsafe {
            let header = &*(self.mmap.as_ptr() as *const ShmHeader);
            (
                header.stats.total_writes.load(Ordering::Relaxed),
                header.stats.total_reads.load(Ordering::Relaxed)
            )
        }
    }
}
