// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Example: Dump AST structure with canonical kinds

#[cfg(feature = "cst_ts")]
use lancedb::processors::cst_to_ast_pipeline::{CstToAstPipeline, AstNode, AstNodeType};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "dump_ast", about = "Dump AST structure from source file")]
struct Opt {
    /// Source file to parse
    #[structopt(parse(from_os_str))]
    file: PathBuf,

    /// Maximum depth to display (default: unlimited)
    #[structopt(short, long)]
    max_depth: Option<usize>,

    /// Show only nodes with identifiers
    #[structopt(short = "i", long)]
    identifiers_only: bool,

    /// Show complexity metrics
    #[structopt(short = "c", long)]
    show_complexity: bool,
}

#[cfg(feature = "cst_ts")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    println!("🔍 Parsing file: {}", opt.file.display());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let pipeline = CstToAstPipeline::new();
    let result = pipeline.process_file(&opt.file).await?;

    // Print metadata
    println!("📊 Metadata:");
    println!("  Language:      {}", result.language);
    println!("  Parse time:    {:.2}ms", result.parse_time_ms);
    println!("  Transform time: {:.2}ms", result.transform_time_ms);
    println!("  Root node:     {:?}", result.ast.node_type);
    println!();

    // Print AST tree
    println!("🌳 AST Structure:\n");
    print_ast_node(
        &result.ast,
        0,
        opt.max_depth,
        opt.identifiers_only,
        opt.show_complexity,
    );

    // Print summary statistics
    println!("\n📈 Statistics:");
    let stats = collect_stats(&result.ast);
    println!("  Total nodes:        {}", stats.total_nodes);
    println!("  Function decls:     {}", stats.function_count);
    println!("  Class decls:        {}", stats.class_count);
    println!("  Variable decls:     {}", stats.variable_count);
    println!("  Max depth:          {}", stats.max_depth);
    if opt.show_complexity {
        println!("  Total complexity:   {}", stats.total_complexity);
    }

    Ok(())
}

#[cfg(feature = "cst_ts")]
fn print_ast_node(
    node: &AstNode,
    depth: usize,
    max_depth: Option<usize>,
    identifiers_only: bool,
    show_complexity: bool,
) {
    // Check depth limit
    if let Some(max) = max_depth {
        if depth >= max {
            return;
        }
    }

    // Skip if filtering by identifiers
    if identifiers_only && node.identifier.is_none() {
        for child in &node.children {
            print_ast_node(child, depth, max_depth, identifiers_only, show_complexity);
        }
        return;
    }

    // Print indentation
    let indent = "  ".repeat(depth);

    // Format node type with color emoji
    let emoji = match node.node_type {
        AstNodeType::FunctionDeclaration => "🔵",
        AstNodeType::ClassDeclaration => "🟢",
        AstNodeType::InterfaceDeclaration => "🟡",
        AstNodeType::StructDeclaration => "🟣",
        AstNodeType::EnumDeclaration => "🟠",
        AstNodeType::VariableDeclaration => "🔷",
        AstNodeType::IfStatement => "❓",
        AstNodeType::ForLoop | AstNodeType::WhileLoop => "🔁",
        AstNodeType::ImportStatement => "📥",
        AstNodeType::ExportStatement => "📤",
        _ => "▪️",
    };

    // Build output string
    let mut output = format!("{}{} {:?}", indent, emoji, node.node_type);

    if let Some(id) = &node.identifier {
        output.push_str(&format!(" '{}'", id));
    }

    if show_complexity && node.metadata.complexity > 1 {
        output.push_str(&format!(" [complexity: {}]", node.metadata.complexity));
    }

    // Show line range for declarations
    if matches!(
        node.node_type,
        AstNodeType::FunctionDeclaration
            | AstNodeType::ClassDeclaration
            | AstNodeType::StructDeclaration
    ) {
        output.push_str(&format!(
            " (lines {}-{})",
            node.metadata.start_line, node.metadata.end_line
        ));
    }

    println!("{}", output);

    // Recursively print children
    for child in &node.children {
        print_ast_node(child, depth + 1, max_depth, identifiers_only, show_complexity);
    }
}

#[cfg(feature = "cst_ts")]
struct AstStats {
    total_nodes: usize,
    function_count: usize,
    class_count: usize,
    variable_count: usize,
    max_depth: usize,
    total_complexity: usize,
}

#[cfg(feature = "cst_ts")]
fn collect_stats(node: &AstNode) -> AstStats {
    let mut stats = AstStats {
        total_nodes: 0,
        function_count: 0,
        class_count: 0,
        variable_count: 0,
        max_depth: 0,
        total_complexity: 0,
    };

    collect_stats_recursive(node, 0, &mut stats);
    stats
}

#[cfg(feature = "cst_ts")]
fn collect_stats_recursive(node: &AstNode, depth: usize, stats: &mut AstStats) {
    stats.total_nodes += 1;
    stats.max_depth = stats.max_depth.max(depth);
    stats.total_complexity += node.metadata.complexity;

    match node.node_type {
        AstNodeType::FunctionDeclaration => stats.function_count += 1,
        AstNodeType::ClassDeclaration => stats.class_count += 1,
        AstNodeType::VariableDeclaration => stats.variable_count += 1,
        _ => {}
    }

    for child in &node.children {
        collect_stats_recursive(child, depth + 1, stats);
    }
}

#[cfg(not(feature = "cst_ts"))]
fn main() {
    eprintln!("Error: This example requires the 'cst_ts' feature to be enabled.");
    eprintln!("Please rebuild with: cargo run --example dump_ast --features cst_ts -- <file>");
    std::process::exit(1);
}
