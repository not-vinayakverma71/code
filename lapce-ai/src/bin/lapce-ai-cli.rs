// CLI Harness for Tool Testing - No IPC required
// Part of CLI harness TODO #16

use anyhow::{Result, Context};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::path::PathBuf;
use lapce_ai_rust::core::tools::{
    traits::{Tool, ToolContext},
    registry::ToolRegistry,
    fs::{
        read_file::ReadFileTool,
        write_file::WriteFileTool,
        search_and_replace::SearchAndReplaceTool,
        search_files::SearchFilesTool,
    },
    diff_tool::DiffTool,
};

#[derive(Parser)]
#[command(name = "lapce-ai-cli")]
#[command(about = "CLI harness for testing lapce-ai tools", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Workspace directory
    #[arg(short, long, default_value = ".")]
    workspace: PathBuf,
    
    /// User ID for context
    #[arg(short, long, default_value = "cli-user")]
    user: String,
    
    /// Output format
    #[arg(short, long, default_value = "json")]
    format: OutputFormat,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum OutputFormat {
    Json,
    Pretty,
    Compact,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "json" => Ok(OutputFormat::Json),
            "pretty" => Ok(OutputFormat::Pretty),
            "compact" => Ok(OutputFormat::Compact),
            _ => Err(format!("Invalid format: {}", s)),
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a tool by name
    Tool {
        /// Tool name
        name: String,
        
        /// Tool arguments as JSON
        #[arg(short, long)]
        args: String,
        
        /// Require approval (for testing approval flow)
        #[arg(long)]
        require_approval: bool,
    },
    
    /// List available tools
    List,
    
    /// Run a test suite
    Test {
        /// Test suite name
        #[arg(default_value = "basic")]
        suite: String,
    },
    
    /// Batch execute tools from JSON file
    Batch {
        /// Input file with tool calls
        file: PathBuf,
        
        /// Continue on error
        #[arg(long)]
        continue_on_error: bool,
    },
    
    /// Show tool documentation
    Doc {
        /// Tool name
        name: String,
    },
    
    /// Benchmark tool performance
    Bench {
        /// Tool name
        name: String,
        
        /// Number of iterations
        #[arg(short, long, default_value = "100")]
        iterations: u32,
        
        /// Arguments for benchmark
        #[arg(short, long, default_value = "{}")]
        args: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    if cli.verbose {
        env_logger::init_from_env(env_logger::Env::new().default_filter_or("debug"));
    } else {
        env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    }
    
    let context = ToolContext::new(cli.workspace.clone(), cli.user.clone());
    
    match cli.command {
        Commands::Tool { name, args, require_approval } => {
            execute_tool(&name, &args, context, require_approval, &cli.format).await?;
        }
        Commands::List => {
            list_tools(&cli.format);
        }
        Commands::Test { suite } => {
            run_test_suite(&suite, context, &cli.format).await?;
        }
        Commands::Batch { file, continue_on_error } => {
            execute_batch(&file, context, continue_on_error, &cli.format).await?;
        }
        Commands::Doc { name } => {
            show_documentation(&name);
        }
        Commands::Bench { name, iterations, args } => {
            benchmark_tool(&name, &args, iterations, context, &cli.format).await?;
        }
    }
    
    Ok(())
}

async fn execute_tool(
    name: &str,
    args_str: &str,
    mut context: ToolContext,
    require_approval: bool,
    format: &OutputFormat,
) -> Result<()> {
    context.require_approval = require_approval;
    
    let args: Value = serde_json::from_str(args_str)
        .context("Invalid JSON arguments")?;
    
    let tool = get_tool(name)?;
    
    let start = std::time::Instant::now();
    let result = tool.execute(args.clone(), context).await;
    let duration = start.elapsed();
    
    let output = match result {
        Ok(tool_result) => {
            json!({
                "success": true,
                "tool": name,
                "args": args,
                "result": tool_result,
                "duration_ms": duration.as_millis(),
            })
        }
        Err(e) => {
            json!({
                "success": false,
                "tool": name,
                "args": args,
                "error": e.to_string(),
                "duration_ms": duration.as_millis(),
            })
        }
    };
    
    print_output(&output, format);
    Ok(())
}

fn list_tools(format: &OutputFormat) {
    let tools = vec![
        ("read_file", "Read files with encoding detection and line ranges"),
        ("write_file", "Write files with backup and artifact removal"),
        ("search_and_replace", "Search and replace with preview mode"),
        ("search_files", "Ripgrep-based file search with streaming"),
        ("apply_diff", "Apply diffs with multiple strategies"),
        ("terminal", "Execute terminal commands with safety checks"),
        ("list_files", "List directory contents"),
        ("insert_content", "Insert content at specific lines"),
    ];
    
    let output = json!({
        "tools": tools.iter().map(|(name, desc)| {
            json!({
                "name": name,
                "description": desc,
            })
        }).collect::<Vec<_>>(),
    });
    
    print_output(&output, format);
}

async fn run_test_suite(suite: &str, context: ToolContext, format: &OutputFormat) -> Result<()> {
    let tests = match suite {
        "basic" => vec![
            ("read_file", json!({"path": "Cargo.toml"})),
            ("list_files", json!({"path": "src", "recursive": false})),
            ("search_files", json!({"query": "TODO", "includes": ["*.rs"]})),
        ],
        "write" => vec![
            ("write_file", json!({
                "path": "test.txt",
                "content": "Hello, World!",
                "createBackup": false,
            })),
            ("read_file", json!({"path": "test.txt"})),
        ],
        _ => {
            anyhow::bail!("Unknown test suite: {}", suite);
        }
    };
    
    let mut results = Vec::new();
    
    for (tool_name, args) in tests {
        let tool = get_tool(tool_name)?;
        let start = std::time::Instant::now();
        let result = tool.execute(args.clone(), context.clone()).await;
        let duration = start.elapsed();
        
        results.push(json!({
            "tool": tool_name,
            "args": args,
            "success": result.is_ok(),
            "duration_ms": duration.as_millis(),
            "error": result.err().map(|e| e.to_string()),
        }));
    }
    
    let output = json!({
        "suite": suite,
        "results": results,
        "total": results.len(),
        "passed": results.iter().filter(|r| r["success"].as_bool().unwrap_or(false)).count(),
    });
    
    print_output(&output, format);
    Ok(())
}

async fn execute_batch(
    file: &PathBuf,
    context: ToolContext,
    continue_on_error: bool,
    format: &OutputFormat,
) -> Result<()> {
    let content = std::fs::read_to_string(file)?;
    let batch: Value = serde_json::from_str(&content)?;
    
    let calls = batch["calls"].as_array()
        .context("Batch file must have 'calls' array")?;
    
    let mut results = Vec::new();
    
    for call in calls {
        let tool_name = call["tool"].as_str()
            .context("Each call must have 'tool' field")?;
        let args = call["args"].clone();
        
        let tool = get_tool(tool_name)?;
        let result = tool.execute(args, context.clone()).await;
        
        if !continue_on_error && result.is_err() {
            anyhow::bail!("Batch execution failed at tool: {}", tool_name);
        }
        
        results.push(json!({
            "tool": tool_name,
            "success": result.is_ok(),
            "error": result.err().map(|e| e.to_string()),
        }));
    }
    
    let output = json!({
        "batch": file,
        "results": results,
    });
    
    print_output(&output, format);
    Ok(())
}

fn show_documentation(name: &str) {
    let doc = match name {
        "read_file" => "Read files with encoding detection and line ranges",
        "write_file" => "Write files with backup and artifact removal",
        "search_files" => "Ripgrep-based file search with streaming",
        _ => "Documentation not available for this tool",
    };
    
    println!("{}", doc);
}

async fn benchmark_tool(
    name: &str,
    args_str: &str,
    iterations: u32,
    context: ToolContext,
    format: &OutputFormat,
) -> Result<()> {
    let args: Value = serde_json::from_str(args_str)?;
    let tool = get_tool(name)?;
    
    let mut durations = Vec::new();
    
    for _ in 0..iterations {
        let start = std::time::Instant::now();
        let _ = tool.execute(args.clone(), context.clone()).await;
        durations.push(start.elapsed().as_millis() as u64);
    }
    
    durations.sort_unstable();
    
    let sum: u64 = durations.iter().sum();
    let avg = sum / iterations as u64;
    let min = durations[0];
    let max = durations[durations.len() - 1];
    let p50 = durations[durations.len() / 2];
    let p95 = durations[durations.len() * 95 / 100];
    let p99 = durations[durations.len() * 99 / 100];
    
    let output = json!({
        "tool": name,
        "iterations": iterations,
        "metrics": {
            "avg_ms": avg,
            "min_ms": min,
            "max_ms": max,
            "p50_ms": p50,
            "p95_ms": p95,
            "p99_ms": p99,
        },
    });
    
    print_output(&output, format);
    Ok(())
}

fn get_tool(name: &str) -> Result<Box<dyn Tool>> {
    match name {
        "read_file" => Ok(Box::new(ReadFileTool)),
        "write_file" => Ok(Box::new(WriteFileTool)),
        "search_and_replace" => Ok(Box::new(SearchAndReplaceTool)),
        "search_files" => Ok(Box::new(SearchFilesTool)),
        "diff" => Ok(Box::new(DiffTool)),
        _ => anyhow::bail!("Unknown tool: {}", name),
    }
}

fn print_output(value: &Value, format: &OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(value).unwrap());
        }
        OutputFormat::Pretty => {
            println!("{}", serde_json::to_string_pretty(value).unwrap());
        }
        OutputFormat::Compact => {
            if value["success"].as_bool().unwrap_or(false) {
                println!("✓ {}", value["tool"].as_str().unwrap_or("unknown"));
            } else {
                println!("✗ {} - {}", 
                    value["tool"].as_str().unwrap_or("unknown"),
                    value["error"].as_str().unwrap_or("error"));
            }
        }
    }
}
