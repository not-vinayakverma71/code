# Release Process

## Version Policy

We follow Semantic Versioning (SemVer):
- **MAJOR**: Breaking API changes
- **MINOR**: New features, backwards compatible
- **PATCH**: Bug fixes, backwards compatible

## Pre-Release Checklist

### Code Quality
- [ ] All tests passing (`cargo test --all-features`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Documentation updated (`cargo doc --no-deps`)
- [ ] CHANGELOG.md updated with all changes

### Performance
- [ ] Benchmarks meet SLO targets
  - Write throughput > 1000 ops/sec
  - Read throughput > 2500 ops/sec
  - P99 latency < 10ms
- [ ] Memory usage within budget (< 100MB for typical workload)
- [ ] No performance regressions vs previous release

### Dependencies
- [ ] Grammar versions pinned in Cargo.toml
- [ ] All dependencies audited (`cargo audit`)
- [ ] License compatibility verified (`cargo deny check`)

### Platform Testing
- [ ] Linux (Ubuntu 20.04+)
- [ ] macOS (11.0+)
- [ ] Windows (10+)

## Release Steps

1. **Update Version**
   ```bash
   # Update version in Cargo.toml
   sed -i 's/version = ".*"/version = "X.Y.Z"/' Cargo.toml
   ```

2. **Update CHANGELOG**
   - Add release date
   - Summarize changes
   - Credit contributors

3. **Create Release Branch**
   ```bash
   git checkout -b release/vX.Y.Z
   git add -A
   git commit -m "chore: prepare release vX.Y.Z"
   ```

4. **Run Final Tests**
   ```bash
   cargo test --all-features --release
   cargo clippy -- -D warnings
   cargo doc --no-deps
   ```

5. **Tag Release**
   ```bash
   git tag -a vX.Y.Z -m "Release version X.Y.Z"
   git push origin release/vX.Y.Z
   git push origin vX.Y.Z
   ```

6. **Publish to crates.io**
   ```bash
   cargo publish --dry-run
   cargo publish
   ```

7. **Create GitHub Release**
   - Go to GitHub releases page
   - Create release from tag
   - Copy CHANGELOG entry
   - Attach pre-built binaries

## Pinned Grammar Versions

Current grammar versions (as of v0.1.0):
- tree-sitter: 0.23.0
- tree-sitter-rust: 0.23.0
- tree-sitter-python: 0.23.2
- tree-sitter-javascript: 0.23.0
- tree-sitter-typescript: 0.23.0
- tree-sitter-go: 0.23.1
- tree-sitter-cpp: 0.23.1
- tree-sitter-c: 0.23.1
- tree-sitter-java: 0.23.2
- tree-sitter-c-sharp: 0.23.0

## Post-Release

1. **Monitor Issues**
   - Watch for bug reports
   - Check performance metrics
   - Monitor crates.io download stats

2. **Update Documentation**
   - Update README with new version
   - Update examples
   - Post release announcement

## Hotfix Process

For critical bugs in released versions:

1. Create hotfix branch from release tag
   ```bash
   git checkout -b hotfix/vX.Y.Z+1 vX.Y.Z
   ```

2. Apply minimal fix
3. Update patch version only
4. Fast-track testing
5. Release as patch version

## Version Support

- **Latest**: Full support
- **Previous minor**: Security fixes only
- **Older versions**: No support

## Contact

Release Manager: @maintainer
Security Issues: security@example.com
