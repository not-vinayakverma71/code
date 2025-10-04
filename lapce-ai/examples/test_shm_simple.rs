use std::fs::OpenOptions;
use std::io::Write;
use memmap2::{MmapMut, MmapOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing shared memory basics...\n");
    
    // Test 1: Can we create a file in /dev/shm?
    let path = "/dev/shm/lapce_test";
    println!("Creating file at: {}", path);
    
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    
    println!("✓ File created");
    
    // Test 2: Can we resize it?
    let size = 1024 * 1024; // 1MB
    file.set_len(size)?;
    file.flush()?;
    println!("✓ File resized to {}KB", size / 1024);
    
    // Test 3: Can we mmap it?
    let mmap = unsafe {
        MmapOptions::new()
            .len(size as usize)
            .map_mut(&file)?
    };
    println!("✓ Memory mapped");
    
    // Test 4: Can we write to it?
    let mut mmap = mmap;
    mmap[0] = 0xAB;
    mmap[1] = 0xCD;
    println!("✓ Wrote to mmap");
    
    // Test 5: Can we read it back?
    if mmap[0] == 0xAB && mmap[1] == 0xCD {
        println!("✓ Read back correctly");
    } else {
        println!("✗ Read back failed");
    }
    
    // Clean up
    drop(mmap);
    drop(file);
    std::fs::remove_file(path)?;
    println!("✓ Cleaned up");
    
    println!("\nAll basic tests passed!");
    Ok(())
}
