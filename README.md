<h1 align="center">
  <a href="https://lapce.dev" target="_blank">
  <img src="extra/images/logo.png" width=200 height=200/><br>
  Lapce
  </a>
</h1>

<h4 align="center">Lightning-fast And Powerful Code Editor</h4>

<div align="center">
  <a href="https://github.com/lapce/lapce/actions/workflows/ci.yml" target="_blank">
    <img src="https://github.com/lapce/lapce/actions/workflows/ci.yml/badge.svg" />
  </a>
  <a href="https://discord.gg/n8tGJ6Rn6D" target="_blank">
    <img src="https://img.shields.io/discord/946858761413328946?logo=discord" />
  </a>
  <a href="https://docs.lapce.dev" target="_blank">
      <img src="https://img.shields.io/static/v1?label=Docs&message=docs.lapce.dev&color=blue" alt="Lapce Docs">
  </a>
  <a href="https://wiki.mutable.ai/lapce/lapce" target="_blank">
      <img src="https://img.shields.io/static/v1?label=Code Wiki&message=Mutable.ai&color=blue" alt="Mutable.ai Auto Wiki">
  </a>
</div>
<br/>


Lapce (IPA: /læps/) is written in pure Rust, with a UI in [Floem](https://github.com/lapce/floem). It is designed with [Rope Science](https://xi-editor.io/docs/rope_science_00.html) from the [Xi-Editor](https://github.com/xi-editor/xi-editor), enabling lightning-fast computation, and leverages [wgpu](https://github.com/gfx-rs/wgpu) for rendering. More information about the features of Lapce can be found on the [main website](https://lapce.dev) and user documentation can be found on [GitBook](https://docs.lapce.dev/).

![](https://github.com/lapce/lapce/blob/master/extra/images/screenshot.png?raw=true)

## Features

* Built-in LSP ([Language Server Protocol](https://microsoft.github.io/language-server-protocol/)) support to give you intelligent code features such as: completion, diagnostics and code actions
* Modal editing support as first class citizen (Vim-like, and toggleable)
* Built-in remote development support inspired by [VSCode Remote Development](https://code.visualstudio.com/docs/remote/remote-overview). Enjoy the benefits of a "local" experience, and seamlessly gain the full power of a remote system. We also have [Lapdev](https://lap.dev/) which can help manage your remote dev environments. 
* Plugins can be written in programming languages that can compile to the [WASI](https://wasi.dev/) format (C, Rust, [AssemblyScript](https://www.assemblyscript.org/))
* Built-in terminal, so you can execute commands in your workspace, without leaving Lapce.
* **AI-Powered Tools**: Integrated AI assistant with secure tool execution for file operations, code editing, and command execution

## AI Tools System

Lapce includes a powerful AI tools execution system that enables AI assistants to safely interact with your codebase.

### Tool Categories

#### Filesystem Tools
- **ReadFile**: Read file contents with optional line ranges
- **WriteFile**: Create or overwrite files with approval
- **EditFile**: Replace content in existing files
- **ListFiles**: List directory contents with glob patterns
- **SearchFiles**: Search text/regex patterns across files

#### Execution Tools
- **ExecuteCommand**: Run shell commands with safety checks
- **DiffTool**: Preview and apply code changes with unified diffs

### Safety Features

#### Approval System
All destructive operations require explicit user approval:
- File writes and modifications
- Command execution
- Diff application

#### .rooignore Support
Control AI access to your filesystem with `.rooignore` files (similar to `.gitignore`):
```
# Block sensitive directories
.env
secrets/
*.key

# Allow specific files
!.env.example
```

#### Security Gates
- **Workspace Bounds**: All paths are validated to prevent directory traversal
- **Command Filtering**: Dangerous commands (rm, sudo, etc.) are blocked
- **Permission System**: Fine-grained control over read/write/execute permissions
- **Timeout Protection**: Commands have configurable timeouts (default: 30s)

### Tool Usage

Tools communicate via XML arguments for clarity:

```xml
<!-- Read a file -->
<tool>
    <path>src/main.rs</path>
    <lineStart>10</lineStart>
    <lineEnd>20</lineEnd>
</tool>

<!-- Execute a command -->
<tool>
    <command>cargo test</command>
    <cwd>./project</cwd>
    <timeout>60</timeout>
</tool>
```

### Performance

All tools meet strict performance budgets:
- File operations: <50ms for 10MB files
- Directory listing: <100ms for 2000 paths
- Command execution: Streaming output with <5ms latency
- Diff application: <100ms for 1000-line files

## Installation

You can find pre-built releases for Windows, Linux and macOS [here](https://github.com/lapce/lapce/releases), or [installing with a package manager](docs/installing-with-package-manager.md).
If you'd like to compile from source, you can find the [guide](docs/building-from-source.md).

## Contributing

<a href="https://ws.lap.dev/#https://github.com/lapce/lapce" target="_blank">
      <img src="https://lap.dev/images/open-in-lapdev.svg?version=8" alt="Open in Lapdev">
</a>

[Lapdev](https://lap.dev/), developed by the Lapce team, is a cloud dev env service similar to GitHub Codespaces. By clicking the button above, you'll be taken to a fully set up Lapce dev env where you can browse the code and start developing. All dependencies are pre-installed, so you can get straight to code.

Guidelines for contributing to Lapce can be found in [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Feedback & Contact

The most popular place for Lapce developers and users is on the [Discord server](https://discord.gg/n8tGJ6Rn6D).

Or, join the discussion on [Reddit](https://www.reddit.com/r/lapce/) where we are just getting started.

There is also a [Matrix Space](https://matrix.to/#/#lapce-editor:matrix.org), which is linked to the content from the Discord server.

## License

Lapce is released under the Apache License Version 2, which is an open source license. You may contribute to this project, or use the code as you please as long as you adhere to its conditions. You can find a copy of the license text here: [`LICENSE`](LICENSE).
