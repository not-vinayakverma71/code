# Step 21: Production Deployment & Packaging
## Single Binary Distribution with Verification

## ⚠️ CRITICAL: FINAL VERIFICATION BEFORE RELEASE
**100% BEHAVIOR MATCH WITH CODEX REQUIRED**

**DEPLOYMENT CHECKLIST**:
- Every TypeScript file has Rust equivalent
- All tests pass (CHARACTER-FOR-CHARACTER)
- Memory < 350MB (vs 3.5GB current)
- Performance 10x better minimum
- Zero behavior regressions

## ✅ Success Criteria
- [ ] **Binary Size**: < 25MB compressed
- [ ] **Static Linking**: Single executable, no deps
- [ ] **Cross-Platform**: Linux, macOS, Windows
- [ ] **Auto-Update**: Built-in update mechanism
- [ ] **Telemetry**: Performance monitoring (opt-in)
- [ ] **Crash Reporting**: Automatic error collection
- [ ] **Rollback**: Previous version recovery
- [ ] **Production Test**: 24h continuous operation

## Overview
Final packaging and deployment of the complete Rust AI IDE.

## Binary Compilation

### Release Build Configuration
```toml
# Cargo.toml
[profile.release]
opt-level = "z"          # Size optimization
lto = true              # Link-time optimization
codegen-units = 1       # Single codegen unit
strip = true            # Strip debug symbols
panic = "abort"         # Smaller panic handler
overflow-checks = false # Disable in release

[profile.release.package."*"]
opt-level = "z"         # Optimize all dependencies

# Platform-specific
[target.'cfg(target_os = "linux")']
rustflags = ["-C", "link-arg=-s"]  # Strip symbols

[target.'cfg(target_os = "macos")']
rustflags = ["-C", "link-arg=-Wl,-dead_strip"]

[target.'cfg(target_os = "windows")']
rustflags = ["-C", "link-arg=/OPT:REF"]
```

### Static Compilation
```rust
// build.rs
fn main() {
    // Static linking for deployment
    println!("cargo:rustc-link-arg=-static");
    
    // Include resources in binary
    include_dir::include_dir!("./resources");
    
    // Embed version info
    println!("cargo:rustc-env=VERSION={}", env!("CARGO_PKG_VERSION"));
}
```

## Cross-Platform Building

### GitHub Actions CI/CD
```yaml
name: Release Build

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        
    runs-on: ${{ matrix.os }}
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        
      - name: Build Release
        run: cargo build --release
        
      - name: Compress Binary
        run: |
          if [[ "$RUNNER_OS" == "Linux" ]]; then
            upx --best --lzma target/release/lapce-ai
          elif [[ "$RUNNER_OS" == "macOS" ]]; then
            gzip -9 target/release/lapce-ai
          elif [[ "$RUNNER_OS" == "Windows" ]]; then
            7z a -mx9 lapce-ai.zip target/release/lapce-ai.exe
          fi
          
      - name: Upload Artifacts
        uses: actions/upload-artifact@v3
        with:
          name: lapce-ai-${{ matrix.os }}
          path: target/release/lapce-ai*
```

## Auto-Update System

```rust
use self_update::cargo_crate_version;

pub struct AutoUpdater {
    current_version: String,
    update_url: String,
}

impl AutoUpdater {
    pub async fn check_for_updates(&self) -> Result<Option<Version>> {
        let releases = self.fetch_releases().await?;
        
        let latest = releases
            .iter()
            .map(|r| Version::parse(&r.version).unwrap())
            .max()
            .unwrap();
            
        let current = Version::parse(&self.current_version).unwrap();
        
        if latest > current {
            Some(latest)
        } else {
            None
        }
    }
    
    pub async fn self_update(&self) -> Result<()> {
        self_update::backends::github::Update::configure()
            .repo_owner("lapce")
            .repo_name("lapce-ai-rust")
            .bin_name("lapce-ai")
            .show_download_progress(true)
            .current_version(cargo_crate_version!())
            .build()?
            .update()?;
            
        Ok(())
    }
}
```

## Telemetry & Monitoring

```rust
use sentry::{ClientOptions, IntoDsn};

pub struct Telemetry {
    enabled: bool,
    client: Option<sentry::Client>,
}

impl Telemetry {
    pub fn init(opt_in: bool) -> Self {
        if !opt_in {
            return Self { enabled: false, client: None };
        }
        
        let client = sentry::Client::new(ClientOptions {
            dsn: "YOUR_SENTRY_DSN".into_dsn().ok(),
            release: Some(env!("VERSION").into()),
            environment: Some("production".into()),
            ..Default::default()
        });
        
        Self {
            enabled: true,
            client: Some(client),
        }
    }
    
    pub fn record_performance(&self, operation: &str, duration: Duration) {
        if !self.enabled { return; }
        
        // Send performance metrics
        if let Some(client) = &self.client {
            let transaction = sentry::start_transaction(
                sentry::TransactionContext::new(operation, "performance"),
            );
            transaction.finish();
        }
    }
}
```

## Crash Reporting

```rust
use backtrace::Backtrace;
use std::panic;

pub fn setup_crash_handler() {
    panic::set_hook(Box::new(|panic_info| {
        let backtrace = Backtrace::new();
        
        // Log locally
        eprintln!("PANIC: {}", panic_info);
        eprintln!("Backtrace:\n{:?}", backtrace);
        
        // Send crash report (if telemetry enabled)
        if let Ok(telemetry) = TELEMETRY.try_lock() {
            telemetry.report_crash(panic_info, backtrace);
        }
        
        // Save crash dump
        save_crash_dump(panic_info, backtrace);
    }));
}

fn save_crash_dump(info: &PanicInfo, backtrace: Backtrace) {
    let crash_dir = dirs::data_dir()
        .unwrap()
        .join("lapce-ai")
        .join("crashes");
        
    fs::create_dir_all(&crash_dir).ok();
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
        
    let crash_file = crash_dir.join(format!("crash_{}.txt", timestamp));
    
    let mut file = File::create(crash_file).unwrap();
    writeln!(file, "Version: {}", env!("VERSION")).ok();
    writeln!(file, "Panic: {}", info).ok();
    writeln!(file, "Backtrace:\n{:?}", backtrace).ok();
}
```

## Distribution Packages

### Debian/Ubuntu Package
```bash
# debian/control
Package: lapce-ai
Version: 1.0.0
Architecture: amd64
Maintainer: Lapce Team
Description: Native AI IDE with minimal memory usage
Depends: libc6 (>= 2.31)
```

### macOS DMG
```bash
# Create app bundle
mkdir -p Lapce-AI.app/Contents/MacOS
cp target/release/lapce-ai Lapce-AI.app/Contents/MacOS/

# Create DMG
hdiutil create -volname "Lapce AI" -srcfolder Lapce-AI.app -ov lapce-ai.dmg
```

### Windows Installer
```nsis
; installer.nsi
Name "Lapce AI"
OutFile "lapce-ai-installer.exe"

InstallDir "$PROGRAMFILES\Lapce AI"

Section
    SetOutPath $INSTDIR
    File "target\release\lapce-ai.exe"
    
    CreateShortcut "$DESKTOP\Lapce AI.lnk" "$INSTDIR\lapce-ai.exe"
SectionEnd
```

## Production Verification

```rust
#[cfg(test)]
mod production_tests {
    #[test]
    fn verify_all_typescript_ported() {
        let ts_files = count_typescript_files("codex-reference/");
        let rust_files = count_rust_implementations("src/");
        
        assert_eq!(
            ts_files, rust_files,
            "Not all TypeScript files ported to Rust"
        );
    }
    
    #[test]
    fn verify_behavior_identical() {
        let test_suite = load_production_test_suite();
        
        for test in test_suite {
            let rust_output = run_rust_implementation(test.input);
            let ts_output = test.expected_output;
            
            assert_eq!(
                rust_output, ts_output,
                "Behavior mismatch in: {}", test.name
            );
        }
    }
    
    #[tokio::test]
    async fn stress_test_24_hours() {
        let start = Instant::now();
        let duration = Duration::from_secs(24 * 60 * 60);
        
        while start.elapsed() < duration {
            // Run continuous operations
            process_random_requests().await;
            
            // Check memory
            let memory = get_memory_usage();
            assert!(memory < 350 * 1024 * 1024, "Memory leak detected");
            
            // Check for crashes
            assert!(!has_crashed(), "System crashed during stress test");
            
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}
```

## Release Checklist

```markdown
## Pre-Release Checklist

### Code Verification
- [ ] All TypeScript files ported to Rust
- [ ] All tests passing (unit, integration, e2e)
- [ ] No compilation warnings
- [ ] No clippy warnings
- [ ] Security audit passed (`cargo audit`)

### Performance Verification
- [ ] Memory < 350MB in production
- [ ] Startup time < 2 seconds
- [ ] Response time < 50ms
- [ ] 10x faster than Node.js version

### Behavior Verification
- [ ] All 29 tools work identically
- [ ] All 8 providers work identically
- [ ] Context management identical
- [ ] Error messages match exactly

### Production Testing
- [ ] 24-hour stress test passed
- [ ] No memory leaks detected
- [ ] No crashes or panics
- [ ] Performance stable over time

### Distribution
- [ ] Linux binary < 25MB
- [ ] macOS binary < 25MB
- [ ] Windows binary < 25MB
- [ ] Auto-updater tested
- [ ] Telemetry opt-in working

### Documentation
- [ ] README updated
- [ ] Migration guide written
- [ ] API documentation complete
- [ ] Performance benchmarks published
```

## Implementation Checklist
- [ ] Configure release build
- [ ] Setup CI/CD pipeline
- [ ] Implement auto-updater
- [ ] Add telemetry (opt-in)
- [ ] Setup crash reporting
- [ ] Create distribution packages
- [ ] Run 24h stress test
- [ ] Final behavior verification
