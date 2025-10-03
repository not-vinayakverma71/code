# Phase 8: Production Deployment & Packaging (1 week)
## Ship-Ready Native AI IDE with < 25MB Binary

## 🎯 STRICT SUCCESS CRITERIA - MUST ACHIEVE ALL
- [ ] **Binary Size**: < 25MB compressed single binary
- [ ] **Total Memory**: < 350MB runtime (vs current 3.5GB = 90% reduction)
- [ ] **Cold Start**: < 2 seconds from launch to ready
- [ ] **AI Parity**: 100% identical behavior to Codex (zero regressions)
- [ ] **All Tools**: 29 tools work perfectly (100% pass rate)
- [ ] **All Providers**: 8 providers work identically to Codex
- [ ] **Performance**: 10x faster than current system minimum
- [ ] **Production Test**: 24h stress test with zero crashes/leaks

⚠️ **GATE**: Release ONLY when all automated tests pass AND manual verification confirms IDENTICAL AI behavior.

## ⚠️ FINAL VERIFICATION: TYPESCRIPT → RUST TRANSLATION COMPLETE
**THIS IS A 1:1 PORT OF YEARS OF BATTLE-TESTED AI**

**BEFORE RELEASE - VERIFY TRANSLATION ACCURACY**:
1. Every file in `/home/verma/lapce/lapce-ai-rust/codex-reference/` has Rust equivalent
2. Line-by-line translation verified
3. Same inputs → Same outputs
4. No "improvements" or "optimizations" made
5. ONLY language changed from TypeScript to Rust

**TRANSLATION CHECKLIST**:
- [ ] All TypeScript functions → Rust functions (same names, snake_case)
- [ ] All TypeScript classes → Rust structs with impl
- [ ] All algorithms IDENTICAL (just syntax different)
- [ ] All error messages CHARACTER-FOR-CHARACTER same
- [ ] All prompts EXACTLY preserved
- [ ] All tool schemas unchanged
- [ ] Years of work preserved perfectly

### Day 1-2: Binary Optimization & Packaging
**Goal:** Single static binary with all features
**Target Size:** < 25MB compressed

### Binary Size Optimization
```rust
// Cargo.toml optimizations
[profile.release]
opt-level = "z"          # Optimize for size
lto = true              # Link-time optimization
codegen-units = 1       # Single codegen unit
strip = true            # Strip symbols
panic = "abort"         # No unwinding code
overflow-checks = false # Remove overflow checks in release

[profile.release.package."*"]
opt-level = "z"         # Optimize all dependencies for size

// Build script (build.rs)
use std::env;
use std::process::Command;

fn main() {
    // Use system allocator to save ~300KB
    if env::var("CARGO_FEATURE_SYSTEM_ALLOC").is_ok() {
        println!("cargo:rustc-link-arg=-nostartfiles");
    }
    
    // Compress embedded resources
    compress_resources();
    
    // Generate version info
    let version = env!("CARGO_PKG_VERSION");
    let commit = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
        
    println!("cargo:rustc-env=BUILD_VERSION={}-{}", version, commit);
}

fn compress_resources() {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    
    // Compress syntax highlighting queries
    let queries_dir = Path::new("resources/queries");
    for entry in fs::read_dir(queries_dir)? {
        let path = entry?.path();
        if path.extension() == Some(OsStr::new("scm")) {
            let compressed = compress_file(&path)?;
            let out_path = path.with_extension("scm.gz");
            fs::write(out_path, compressed)?;
        }
    }
}
```

### Platform-Specific Packaging
```rust
pub struct PackageBuilder {
    target_os: TargetOS,
    architecture: Architecture,
    features: Vec<String>,
}

impl PackageBuilder {
    pub async fn build_release(&self) -> Result<PackageInfo> {
        match self.target_os {
            TargetOS::Linux => self.build_linux().await,
            TargetOS::MacOS => self.build_macos().await,
            TargetOS::Windows => self.build_windows().await,
        }
    }
    
    async fn build_linux(&self) -> Result<PackageInfo> {
        // Build static binary with musl
        Command::new("cargo")
            .args(&[
                "build",
                "--release",
                "--target", "x86_64-unknown-linux-musl",
                "--features", "static-link",
            ])
            .status()?;
            
        // Create AppImage
        self.create_appimage().await?;
        
        // Create .deb package
        self.create_deb_package().await?;
        
        // Create .rpm package
        self.create_rpm_package().await?;
        
        // Create tarball
        self.create_tarball().await?;
        
        Ok(PackageInfo {
            binary_size: fs::metadata("target/release/lapce-ai")?.len(),
            compressed_size: fs::metadata("dist/lapce-ai.tar.gz")?.len(),
            formats: vec!["AppImage", "deb", "rpm", "tar.gz"],
        })
    }
    
    async fn create_appimage(&self) -> Result<()> {
        // AppImage structure
        let appdir = PathBuf::from("AppDir");
        fs::create_dir_all(&appdir)?;
        
        // Copy binary
        fs::copy(
            "target/release/lapce-ai",
            appdir.join("usr/bin/lapce-ai"),
        )?;
        
        // Create desktop entry
        fs::write(
            appdir.join("lapce-ai.desktop"),
            r#"[Desktop Entry]
Type=Application
Name=Lapce AI
Exec=lapce-ai
Icon=lapce-ai
Categories=Development;IDE;
"#,
        )?;
        
        // Create AppRun script
        fs::write(
            appdir.join("AppRun"),
            r#"#!/bin/sh
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
export PATH="${HERE}/usr/bin:${PATH}"
exec "${HERE}/usr/bin/lapce-ai" "$@"
"#,
        )?;
        
        // Build AppImage
        Command::new("appimagetool")
            .args(&[appdir.as_os_str(), OsStr::new("lapce-ai.AppImage")])
            .status()?;
            
        Ok(())
    }
}
```

### Day 3-4: Auto-Update System
**Goal:** Seamless updates without user intervention
**Memory Target:** < 1MB overhead

```rust
pub struct AutoUpdater {
    current_version: Version,
    update_channel: UpdateChannel,
    signature_verifier: Arc<SignatureVerifier>,
}

impl AutoUpdater {
    pub async fn check_for_updates(&self) -> Result<Option<UpdateInfo>> {
        let update_url = format!(
            "https://releases.lapce.dev/{}/latest.json",
            self.update_channel
        );
        
        let response = reqwest::get(&update_url).await?;
        let latest: UpdateManifest = response.json().await?;
        
        if latest.version > self.current_version {
            // Verify signature
            if !self.signature_verifier.verify(&latest)? {
                return Err(anyhow!("Invalid update signature"));
            }
            
            return Ok(Some(UpdateInfo {
                version: latest.version,
                download_url: latest.download_url,
                size: latest.size,
                changelog: latest.changelog,
            }));
        }
        
        Ok(None)
    }
    
    pub async fn download_and_apply(&self, update: UpdateInfo) -> Result<()> {
        // Download to temp location
        let temp_path = env::temp_dir().join(format!("lapce-ai-{}.tmp", update.version));
        
        // Stream download with progress
        let response = reqwest::get(&update.download_url).await?;
        let total_size = response.content_length().unwrap_or(0);
        
        let mut file = tokio::fs::File::create(&temp_path).await?;
        let mut stream = response.bytes_stream();
        let mut downloaded = 0u64;
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            
            // Report progress
            self.report_progress(downloaded, total_size);
        }
        
        // Verify downloaded file
        self.verify_download(&temp_path, &update).await?;
        
        // Apply update
        self.apply_update(temp_path).await?;
        
        Ok(())
    }
    
    async fn apply_update(&self, new_binary: PathBuf) -> Result<()> {
        let current_exe = env::current_exe()?;
        let backup = current_exe.with_extension("old");
        
        // Atomic update on Unix
        #[cfg(unix)]
        {
            // Move current to backup
            fs::rename(&current_exe, &backup)?;
            
            // Move new to current
            fs::rename(&new_binary, &current_exe)?;
            
            // Set executable permissions
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&current_exe)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&current_exe, perms)?;
            
            // Schedule restart
            self.schedule_restart().await?;
        }
        
        Ok(())
    }
}
```

### Day 5: Installation & First-Run Experience
**Goal:** Zero-configuration setup
**Target:** < 5 seconds to first AI response

```rust
pub struct Installer {
    platform: Platform,
    config_manager: Arc<ConfigManager>,
}

impl Installer {
    pub async fn install(&self) -> Result<()> {
        // 1. Detect environment
        let env_info = self.detect_environment().await?;
        
        // 2. Create directories
        self.create_directories().await?;
        
        // 3. Install binary
        self.install_binary().await?;
        
        // 4. Setup shell integration
        self.setup_shell_integration().await?;
        
        // 5. Configure IDE integration
        self.configure_ide_integration(&env_info).await?;
        
        // 6. Download minimal AI model
        self.download_bootstrap_model().await?;
        
        Ok(())
    }
    
    async fn setup_shell_integration(&self) -> Result<()> {
        let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        
        let init_script = match shell.as_str() {
            s if s.ends_with("bash") => self.generate_bash_init(),
            s if s.ends_with("zsh") => self.generate_zsh_init(),
            s if s.ends_with("fish") => self.generate_fish_init(),
            _ => return Ok(()),
        };
        
        // Add to shell config
        let config_file = self.get_shell_config_path(&shell)?;
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(config_file)?;
            
        writeln!(file, "\n# Lapce AI integration")?;
        writeln!(file, "{}", init_script)?;
        
        Ok(())
    }
    
    async fn download_bootstrap_model(&self) -> Result<()> {
        // Download minimal 5MB quantized model for offline start
        let model_url = "https://models.lapce.dev/bootstrap/mini-bert-q4.onnx";
        let model_path = self.config_manager.models_dir().join("bootstrap.onnx");
        
        if !model_path.exists() {
            let response = reqwest::get(model_url).await?;
            let bytes = response.bytes().await?;
            tokio::fs::write(&model_path, bytes).await?;
        }
        
        Ok(())
    }
}
```

### Day 6: Production Monitoring & Telemetry
**Goal:** Zero-overhead observability
**Memory Target:** < 500KB

```rust
pub struct ProductionMonitor {
    metrics: Arc<Metrics>,
    crash_reporter: Arc<CrashReporter>,
    health_checker: Arc<HealthChecker>,
}

impl ProductionMonitor {
    pub fn init() -> Self {
        // Install panic hook
        std::panic::set_hook(Box::new(|panic_info| {
            let backtrace = std::backtrace::Backtrace::capture();
            
            // Save crash dump
            if let Ok(dump_path) = save_crash_dump(panic_info, &backtrace) {
                eprintln!("Crash dump saved to: {}", dump_path.display());
            }
            
            // Try to recover
            if let Ok(recovery) = attempt_recovery() {
                eprintln!("Recovery attempted: {:?}", recovery);
            }
        }));
        
        Self {
            metrics: Arc::new(Metrics::new()),
            crash_reporter: Arc::new(CrashReporter::new()),
            health_checker: Arc::new(HealthChecker::new()),
        }
    }
    
    pub async fn health_check(&self) -> HealthStatus {
        let checks = vec![
            self.check_memory_usage(),
            self.check_file_handles(),
            self.check_response_time(),
            self.check_ai_providers(),
        ];
        
        let results = futures::future::join_all(checks).await;
        
        HealthStatus {
            healthy: results.iter().all(|r| r.is_ok()),
            checks: results,
            timestamp: SystemTime::now(),
        }
    }
}

// Minimal telemetry (opt-in)
pub struct Telemetry {
    enabled: bool,
    queue: Arc<Mutex<Vec<TelemetryEvent>>>,
    sender: Arc<TelemetrySender>,
}

impl Telemetry {
    pub fn record(&self, event: TelemetryEvent) {
        if !self.enabled {
            return;
        }
        
        // Privacy-preserving: no PII
        let sanitized = event.sanitize();
        
        // Batch events
        let mut queue = self.queue.lock().unwrap();
        queue.push(sanitized);
        
        if queue.len() >= 100 {
            let batch = std::mem::take(&mut *queue);
            self.sender.send_batch(batch);
        }
    }
}
```

### Day 7: Documentation & Release
**Goal:** Complete user and developer documentation

```rust
/// Generate documentation from code
pub struct DocGenerator {
    parsers: Arc<CodeIntelligence>,
    template_engine: Arc<TemplateEngine>,
}

impl DocGenerator {
    pub async fn generate_docs(&self) -> Result<()> {
        // 1. API documentation
        self.generate_api_docs().await?;
        
        // 2. User guide
        self.generate_user_guide().await?;
        
        // 3. Configuration reference
        self.generate_config_reference().await?;
        
        // 4. Plugin development guide
        self.generate_plugin_guide().await?;
        
        Ok(())
    }
    
    async fn generate_api_docs(&self) -> Result<()> {
        Command::new("cargo")
            .args(&["doc", "--no-deps", "--document-private-items"])
            .status()?;
            
        // Generate additional examples
        let examples = self.extract_code_examples().await?;
        self.template_engine.render_examples(examples).await?;
        
        Ok(())
    }
}
```

## Release Checklist
```rust
pub async fn release_checklist() -> Result<()> {
    let checks = vec![
        ("Memory usage < 50MB", check_memory_usage().await?),
        ("Binary size < 25MB", check_binary_size().await?),
        ("Startup time < 100ms", check_startup_time().await?),
        ("All tests passing", run_all_tests().await?),
        ("No memory leaks", check_memory_leaks().await?),
        ("Documentation complete", check_documentation().await?),
        ("Packages built", check_packages().await?),
        ("Signatures valid", check_signatures().await?),
    ];
    
    for (check, result) in checks {
        println!("✓ {}: {}", check, if result { "PASS" } else { "FAIL" });
        if !result {
            return Err(anyhow!("Release check failed: {}", check));
        }
    }
    
    println!("\n🎉 All checks passed! Ready for release.");
    Ok(())
}
```

## Final Binary Composition
```
lapce-ai (24.8MB total)
├── Core Runtime (2MB)
│   ├── Tokio async runtime
│   └── Basic stdlib
├── IPC & Protocol (3MB)
│   ├── Binary codec
│   └── Unix socket handler
├── AI Providers (8MB)
│   ├── HTTP clients
│   └── 8 provider implementations
├── Tree-sitter Parsers (5MB)
│   └── 8 language parsers (native)
├── Code Intelligence (3MB)
│   ├── Symbol indexing
│   └── Semantic analysis
├── UI Components (2MB)
│   └── Terminal UI (ratatui)
├── Git Integration (1MB)
│   └── libgit2 bindings
└── Misc & Overhead (0.8MB)
```

## Dependencies
```toml
[dependencies]
# Compression
flate2 = "1.0"
zstd = "0.13"

# Updates
semver = "1.0"
reqwest = { version = "0.12", features = ["stream"] }

# Packaging
tar = "0.4"

[build-dependencies]
cc = "1.0"
pkg-config = "0.3"
```

## Expected Results - Phase 8
- **Binary Size**: < 25MB compressed
- **Installation Time**: < 10 seconds
- **First Run**: < 5 seconds to AI ready
- **Update Size**: < 5MB delta updates
- **Platform Coverage**: Linux, macOS, Windows
- **Package Formats**: AppImage, deb, rpm, dmg, msi

## Final System Specifications
- **Total Memory**: 41-46MB (86% reduction from original)
- **Startup Time**: < 100ms
- **First Token**: < 5ms
- **Binary Size**: 25MB
- **Languages Supported**: 8+
- **AI Providers**: 8 major providers
- **100% Feature Complete**: All Codex features + more
