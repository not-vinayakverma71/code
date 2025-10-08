use async_trait::async_trait;
use anyhow::Result;
use serde_json::{json, Value};
use git2::{Repository, Status, DiffOptions};

use crate::mcp_tools::{
    core::{Tool, ToolContext, ToolResult, ToolParameter, ResourceLimits},
    permissions::Permission,
};

// Placeholder for ApplyDiffTool
pub struct ApplyDiffTool;
impl ApplyDiffTool {
    pub fn new() -> Self { Self }
}

pub struct GitTool {
    operations: GitOperations,
}

impl GitTool {
    pub fn new() -> Self {
        Self {
            operations: GitOperations::new(),
        }
    }
}

#[async_trait]
impl Tool for GitTool {
    fn name(&self) -> &str {
        "gitTool"
    }
    
    fn description(&self) -> &str {
        "Git operations including status, diff, commit, branch, and log"
    }
    
    fn parameters(&self) -> Vec<ToolParameter> {
        vec![]
    }
    
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["status", "diff", "commit", "branch", "log", "add", "reset", "push", "pull"]
                },
                "message": {
                    "type": "string",
                    "description": "Commit message"
                },
                "branch": {
                    "type": "string",
                    "description": "Branch name"
                },
                "path": {
                    "type": "string",
                    "description": "File path for specific operations"
                },
                "staged": {
                    "type": "boolean",
                    "description": "Whether to show staged changes"
                },
                "limit": {
                    "type": "integer",
                    "description": "Limit for log entries"
                }
            },
            "required": ["operation"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        let operation = args["operation"].as_str()
            .ok_or_else(|| anyhow::anyhow!("operation required"))?;
        
        match operation {
            "commit" => {
                if args["message"].is_null() {
                    return Err(anyhow::anyhow!("message required for commit"));
                }
            }
            "branch" => {
                if args["branch"].is_null() {
                    return Err(anyhow::anyhow!("branch name required"));
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    async fn execute(&self, args: Value, context: ToolContext) -> Result<ToolResult> {
        let operation = args["operation"].as_str()
            .ok_or_else(|| anyhow::anyhow!("operation required"))?;
        
        match operation {
            "status" => self.operations.git_status(&context).await,
            "diff" => self.operations.git_diff(&args, &context).await,
            "commit" => self.operations.git_commit(&args, &context).await,
            "branch" => self.operations.git_branch(&args, &context).await,
            "log" => self.operations.git_log(&args, &context).await,
            "add" => self.operations.git_add(&args, &context).await,
            "reset" => self.operations.git_reset(&args, &context).await,
            "push" => self.operations.git_push(&args, &context).await,
            "pull" => self.operations.git_pull(&args, &context).await,
            _ => Err(anyhow::anyhow!("Unknown operation: {}", operation)),
        }
    }
    
    fn required_permissions(&self) -> Vec<Permission> {
        vec![
            Permission::FileRead("*".to_string()),
            Permission::FileWrite("*".to_string()),
            Permission::ProcessExecute("git".to_string()),
        ]
    }
    
    fn resource_limits(&self) -> ResourceLimits {
        ResourceLimits {
            max_memory_mb: 100,
            max_cpu_seconds: 30,
            max_file_size_mb: 100,
            max_concurrent_ops: 5,
        }
    }
}

struct GitOperations;

impl GitOperations {
    fn new() -> Self {
        Self
    }
    
    async fn git_status(&self, context: &ToolContext) -> Result<ToolResult> {
        let repo = Repository::open(&context.workspace)?;
        let statuses = repo.statuses(None)?;
        
        let mut files = Vec::new();
        for entry in statuses.iter() {
            let status = entry.status();
            let path = entry.path().unwrap_or("");
            
            files.push(json!({
                "path": path,
                "status": Self::status_to_string(status),
                "staged": status.contains(Status::INDEX_NEW | 
                                         Status::INDEX_MODIFIED | 
                                         Status::INDEX_DELETED),
            }));
        }
        
        Ok(ToolResult::success(json!({ 
            "files": files,
            "branch": repo.head()?.shorthand().unwrap_or("HEAD").to_string()
        })))
    }
    
    async fn git_diff(&self, args: &Value, context: &ToolContext) -> Result<ToolResult> {
        let repo = Repository::open(&context.workspace)?;
        let mut diff_options = DiffOptions::new();
        
        if let Some(path) = args["path"].as_str() {
            diff_options.pathspec(path);
        }
        
        let diff = if args["staged"].as_bool().unwrap_or(false) {
            let head = repo.head()?.peel_to_tree()?;
            let index = repo.index()?;
            repo.diff_tree_to_index(Some(&head), Some(&index), Some(&mut diff_options))?
        } else {
            repo.diff_index_to_workdir(None, Some(&mut diff_options))?
        };
        
        let mut patches = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                patches.push(json!({
                    "old_path": delta.old_file().path().map(|p| p.to_string_lossy()),
                    "new_path": delta.new_file().path().map(|p| p.to_string_lossy()),
                    "status": format!("{:?}", delta.status()),
                }));
                true
            },
            None,
            None,
            None,
        )?;
        
        Ok(ToolResult::success(json!({ "patches": patches })))
    }
    
    async fn git_commit(&self, args: &Value, context: &ToolContext) -> Result<ToolResult> {
        let repo = Repository::open(&context.workspace)?;
        let message = args["message"].as_str()
            .ok_or_else(|| anyhow::anyhow!("message required"))?;
        
        // Get signature
        let sig = repo.signature()?;
        
        // Get parent commit
        let parent_commit = repo.head()?.peel_to_commit()?;
        
        // Get tree from index
        let mut index = repo.index()?;
        let oid = index.write_tree()?;
        let tree = repo.find_tree(oid)?;
        
        // Create commit
        let commit_oid = repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &[&parent_commit],
        )?;
        
        Ok(ToolResult::success(json!({
            "commit_id": commit_oid.to_string(),
            "message": message
        })))
    }
    
    async fn git_branch(&self, args: &Value, context: &ToolContext) -> Result<ToolResult> {
        let repo = Repository::open(&context.workspace)?;
        
        if let Some(branch_name) = args["branch"].as_str() {
            // Create new branch
            let head = repo.head()?.peel_to_commit()?;
            repo.branch(branch_name, &head, false)?;
            
            Ok(ToolResult::success(json!({
                "created": branch_name,
                "from": head.id().to_string()
            })))
        } else {
            // List branches
            let mut branches = Vec::new();
            for branch in repo.branches(None)? {
                let (branch, _) = branch?;
                if let Some(name) = branch.name()? {
                    branches.push(json!({
                        "name": name,
                        "is_head": branch.is_head()
                    }));
                }
            }
            
            Ok(ToolResult::success(json!({ "branches": branches })))
        }
    }
    
    async fn git_log(&self, args: &Value, context: &ToolContext) -> Result<ToolResult> {
        let repo = Repository::open(&context.workspace)?;
        let limit = args["limit"].as_u64().unwrap_or(10) as usize;
        
        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        
        let mut commits = Vec::new();
        for (i, oid) in revwalk.enumerate() {
            if i >= limit {
                break;
            }
            
            let oid = oid?;
            let commit = repo.find_commit(oid)?;
            
            commits.push(json!({
                "id": oid.to_string(),
                "message": commit.message().unwrap_or(""),
                "author": commit.author().name().unwrap_or(""),
                "time": commit.time().seconds()
            }));
        }
        
        Ok(ToolResult::success(json!({ "commits": commits })))
    }
    
    async fn git_add(&self, args: &Value, context: &ToolContext) -> Result<ToolResult> {
        let repo = Repository::open(&context.workspace)?;
        let mut index = repo.index()?;
        
        if let Some(path) = args["path"].as_str() {
            index.add_path(std::path::Path::new(path))?;
        } else {
            index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        }
        
        index.write()?;
        
        Ok(ToolResult::success(json!({
            "status": "Files added to index"
        })))
    }
    
    async fn git_reset(&self, args: &Value, context: &ToolContext) -> Result<ToolResult> {
        let repo = Repository::open(&context.workspace)?;
        let head = repo.head()?.peel_to_commit()?;
        
        repo.reset(head.as_object(), git2::ResetType::Mixed, None)?;
        
        Ok(ToolResult::success(json!({
            "status": "Reset to HEAD"
        })))
    }
    
    async fn git_push(&self, _args: &Value, context: &ToolContext) -> Result<ToolResult> {
        // For push/pull, we'll use the command line git for simplicity
        let output = tokio::process::Command::new("git")
            .arg("push")
            .current_dir(&context.workspace)
            .output()
            .await?;
        
        Ok(ToolResult::success(json!({
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "success": output.status.success()
        })))
    }
    
    async fn git_pull(&self, _args: &Value, context: &ToolContext) -> Result<ToolResult> {
        let output = tokio::process::Command::new("git")
            .arg("pull")
            .current_dir(&context.workspace)
            .output()
            .await?;
        
        Ok(ToolResult::success(json!({
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "success": output.status.success()
        })))
    }
    
    fn status_to_string(status: Status) -> String {
        let mut result = Vec::new();
        
        if status.contains(Status::INDEX_NEW) {
            result.push("new");
        }
        if status.contains(Status::INDEX_MODIFIED) {
            result.push("modified");
        }
        if status.contains(Status::INDEX_DELETED) {
            result.push("deleted");
        }
        if status.contains(Status::WT_NEW) {
            result.push("untracked");
        }
        if status.contains(Status::WT_MODIFIED) {
            result.push("changed");
        }
        if status.contains(Status::WT_DELETED) {
            result.push("missing");
        }
        
        result.join(", ")
    }
}
