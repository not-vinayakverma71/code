use std::path::{Path, PathBuf};
use std::collections::HashSet;
use anyhow::{Result, bail};

pub struct FileSystemGuard {
    workspace: PathBuf,
    allowed_paths: HashSet<PathBuf>,
    denied_paths: HashSet<PathBuf>,
}

impl FileSystemGuard {
    pub fn new(workspace: PathBuf) -> Self {
        let mut denied_paths = HashSet::new();
        denied_paths.insert(PathBuf::from("/etc"));
        denied_paths.insert(PathBuf::from("/root"));
        denied_paths.insert(PathBuf::from("/sys"));
        denied_paths.insert(PathBuf::from("/proc"));
        
        Self {
            workspace,
            allowed_paths: HashSet::new(),
            denied_paths,
        }
    }
    
    pub fn check_read_permission(&self, path: &Path, user: &str) -> Result<()> {
        if !self.is_path_allowed(path) {
            bail!("Read access denied for path: {:?} by user: {}", path, user);
        }
        Ok(())
    }
    
    pub fn check_write_permission(&self, path: &Path, user: &str) -> Result<()> {
        if !self.is_path_allowed(path) {
            bail!("Write access denied for path: {:?} by user: {}", path, user);
        }
        
        // Additional check for write - must be within workspace
        if !path.starts_with(&self.workspace) {
            bail!("Write access only allowed within workspace");
        }
        
        Ok(())
    }
    
    fn is_path_allowed(&self, path: &Path) -> bool {
        // Check if path is in denied list
        for denied in &self.denied_paths {
            if path.starts_with(denied) {
                return false;
            }
        }
        
        // Check if path is explicitly allowed
        if self.allowed_paths.contains(path) {
            return true;
        }
        
        // Default: allow if within workspace
        path.starts_with(&self.workspace)
    }
    
    pub fn add_allowed_path(&mut self, path: PathBuf) {
        self.allowed_paths.insert(path);
    }
    
    pub fn add_denied_path(&mut self, path: PathBuf) {
        self.denied_paths.insert(path);
    }
}
