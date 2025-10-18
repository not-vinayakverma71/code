//! Cache migration tool for upgrading bytecode format versions

use clap::{Parser, Subcommand};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};


#[derive(Parser)]
#[command(name = "migrate-cache")]
#[command(about = "Migrate CST cache to new bytecode format")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check cache format version
    Check {
        /// Path to cache directory
        #[arg(short, long)]
        path: PathBuf,
    },
    
    /// Migrate cache to new format
    Migrate {
        /// Source cache directory
        #[arg(short, long)]
        source: PathBuf,
        
        /// Destination cache directory
        #[arg(short, long)]
        dest: PathBuf,
        
        /// Force overwrite existing files
        #[arg(short, long)]
        force: bool,
        
        /// Dry run - don't actually migrate
        #[arg(long)]
        dry_run: bool,
    },
    
    /// Validate migrated cache
    Validate {
        /// Path to cache directory
        #[arg(short, long)]
        path: PathBuf,
    },
    
    /// Show migration statistics
    Stats {
        /// Path to cache directory
        #[arg(short, long)]
        path: PathBuf,
    },
}

const CURRENT_VERSION: u32 = 2;
const MAGIC_BYTES: [u8; 4] = [0x43, 0x53, 0x54, 0x01]; // "CST\x01"

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Check { path } => check_cache(&path)?,
        Commands::Migrate { source, dest, force, dry_run } => {
            migrate_cache(&source, &dest, force, dry_run)?
        },
        Commands::Validate { path } => validate_cache(&path)?,
        Commands::Stats { path } => show_stats(&path)?,
    }
    
    Ok(())
}

fn check_cache(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Checking cache at: {}", path.display());
    
    let mut total_files = 0;
    let mut v1_files = 0;
    let mut v2_files = 0;
    let mut unknown_files = 0;
    
    for entry in walkdir::WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            total_files += 1;
            
            match detect_version(entry.path()) {
                Ok(1) => v1_files += 1,
                Ok(2) => v2_files += 1,
                _ => unknown_files += 1,
            }
        }
    }
    
    println!("\nCache Statistics:");
    println!("  Total files: {}", total_files);
    println!("  Version 1:   {} files", v1_files);
    println!("  Version 2:   {} files", v2_files);
    println!("  Unknown:     {} files", unknown_files);
    
    if v1_files > 0 {
        println!("\n⚠️  Found {} v1 files that need migration", v1_files);
    } else {
        println!("\n✅ All files are up to date");
    }
    
    Ok(())
}

fn migrate_cache(
    source: &Path,
    dest: &Path,
    force: bool,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Migrating cache:");
    println!("  From: {}", source.display());
    println!("  To:   {}", dest.display());
    
    if dry_run {
        println!("  Mode: DRY RUN (no changes will be made)");
    }
    
    if !source.exists() {
        return Err("Source directory does not exist".into());
    }
    
    if dest.exists() && !force && !dry_run {
        return Err("Destination exists. Use --force to overwrite".into());
    }
    
    let mut migrated = 0;
    let mut skipped = 0;
    let mut errors = 0;
    
    for entry in walkdir::WalkDir::new(source) {
        let entry = entry?;
        
        if entry.file_type().is_file() {
            let src_path = entry.path();
            let rel_path = src_path.strip_prefix(source)?;
            let dst_path = dest.join(rel_path);
            
            match migrate_file(src_path, &dst_path, dry_run) {
                Ok(MigrateResult::Migrated) => {
                    migrated += 1;
                    println!("  ✓ Migrated: {}", rel_path.display());
                }
                Ok(MigrateResult::Skipped) => {
                    skipped += 1;
                }
                Ok(MigrateResult::AlreadyCurrent) => {
                    skipped += 1;
                    if !dry_run {
                        // Copy as-is
                        if let Some(parent) = dst_path.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        fs::copy(src_path, &dst_path)?;
                    }
                }
                Err(e) => {
                    errors += 1;
                    eprintln!("  ✗ Error migrating {}: {}", rel_path.display(), e);
                }
            }
        }
    }
    
    println!("\nMigration Summary:");
    println!("  Migrated: {} files", migrated);
    println!("  Skipped:  {} files", skipped);
    println!("  Errors:   {} files", errors);
    
    if errors > 0 {
        return Err(format!("{} files failed to migrate", errors).into());
    }
    
    Ok(())
}

enum MigrateResult {
    Migrated,
    Skipped,
    AlreadyCurrent,
}

fn migrate_file(
    src: &Path,
    dst: &Path,
    dry_run: bool,
) -> Result<MigrateResult, Box<dyn std::error::Error>> {
    let version = detect_version(src)?;
    
    if version == CURRENT_VERSION {
        return Ok(MigrateResult::AlreadyCurrent);
    }
    
    if version != 1 {
        return Ok(MigrateResult::Skipped);
    }
    
    // Read v1 format
    let v1_data = fs::read(src)?;
    
    // Convert to v2 format
    let v2_data = convert_v1_to_v2(&v1_data)?;
    
    if !dry_run {
        // Write v2 format
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(dst, v2_data)?;
    }
    
    Ok(MigrateResult::Migrated)
}

fn detect_version(path: &Path) -> Result<u32, Box<dyn std::error::Error>> {
    let mut file = fs::File::open(path)?;
    let mut header = [0u8; 8];
    file.read_exact(&mut header)?;
    
    // Check magic bytes
    if header[0..4] == MAGIC_BYTES {
        // Version 2 with header
        let version = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
        Ok(version)
    } else {
        // Version 1 (no header)
        Ok(1)
    }
}

fn convert_v1_to_v2(v1_data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut v2_data = Vec::new();
    
    // Write v2 header
    v2_data.extend_from_slice(&MAGIC_BYTES);
    v2_data.extend_from_slice(&CURRENT_VERSION.to_le_bytes());
    
    // Copy v1 data (assuming compatible format)
    v2_data.extend_from_slice(v1_data);
    
    // Add CRC32 checksum
    let checksum = crc32fast::hash(&v1_data);
    v2_data.extend_from_slice(&checksum.to_le_bytes());
    
    Ok(v2_data)
}

fn validate_cache(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Validating cache at: {}", path.display());
    
    let mut valid = 0;
    let mut invalid = 0;
    
    for entry in walkdir::WalkDir::new(path) {
        let entry = entry?;
        
        if entry.file_type().is_file() {
            match validate_file(entry.path()) {
                Ok(()) => valid += 1,
                Err(e) => {
                    invalid += 1;
                    eprintln!("  ✗ Invalid: {} - {}", entry.path().display(), e);
                }
            }
        }
    }
    
    println!("\nValidation Results:");
    println!("  Valid:   {} files", valid);
    println!("  Invalid: {} files", invalid);
    
    if invalid > 0 {
        return Err(format!("{} files are invalid", invalid).into());
    }
    
    println!("\n✅ All files are valid");
    Ok(())
}

fn validate_file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = fs::read(path)?;
    
    if data.len() < 12 {
        return Err("File too small".into());
    }
    
    // Check header
    if data[0..4] != MAGIC_BYTES {
        return Err("Invalid magic bytes".into());
    }
    
    let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    if version != CURRENT_VERSION {
        return Err(format!("Wrong version: {}", version).into());
    }
    
    // Verify CRC32
    let content_end = data.len() - 4;
    let content = &data[8..content_end];
    let stored_crc = u32::from_le_bytes([
        data[content_end],
        data[content_end + 1],
        data[content_end + 2],
        data[content_end + 3],
    ]);
    
    let computed_crc = crc32fast::hash(content);
    if stored_crc != computed_crc {
        return Err("CRC mismatch".into());
    }
    
    Ok(())
}

fn show_stats(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Cache statistics for: {}", path.display());
    
    let mut total_files = 0;
    let mut total_size = 0u64;
    let mut size_by_ext: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    
    for entry in walkdir::WalkDir::new(path) {
        let entry = entry?;
        
        if entry.file_type().is_file() {
            total_files += 1;
            let size = entry.metadata()?.len();
            total_size += size;
            
            let ext = entry.path()
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("no_ext")
                .to_string();
            
            *size_by_ext.entry(ext).or_insert(0) += size;
        }
    }
    
    println!("\nOverall Statistics:");
    println!("  Total files: {}", total_files);
    println!("  Total size:  {:.2} MB", total_size as f64 / 1_048_576.0);
    println!("  Avg size:    {:.2} KB", (total_size as f64 / total_files as f64) / 1024.0);
    
    println!("\nSize by extension:");
    let mut ext_vec: Vec<_> = size_by_ext.iter().collect();
    ext_vec.sort_by_key(|(_, size)| std::cmp::Reverse(**size));
    
    for (ext, size) in ext_vec.iter().take(10) {
        println!("  .{:<10} {:.2} MB", ext, **size as f64 / 1_048_576.0);
    }
    
    Ok(())
}
