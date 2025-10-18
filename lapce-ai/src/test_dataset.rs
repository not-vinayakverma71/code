// PHASE 1, STEP 1.1: TEST DATASET & GROUND TRUTH STRUCTURES
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruth {
    pub queries: Vec<TestQuery>,
    pub relevance_mappings: HashMap<String, Vec<RelevanceScore>>,
    pub metadata: DatasetMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestQuery {
    pub id: String,
    pub query_text: String,
    pub query_type: QueryType,
    pub expected_results: Vec<ExpectedResult>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    FunctionSearch,      // Looking for specific functions
    ClassSearch,         // Looking for classes/structs
    ImportSearch,        // Looking for imports/dependencies
    AlgorithmSearch,     // Looking for algorithms
    PatternSearch,       // Looking for design patterns
    ErrorHandling,       // Looking for error handling
    Testing,            // Looking for test code
    Documentation,      // Looking for documented code
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedResult {
    pub file_path: String,
    pub relevance: f32,  // 1.0 = perfect match, 0.0 = not relevant
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceScore {
    pub query_id: String,
    pub file_path: String,
    pub relevance: f32,
    pub is_relevant: bool,  // Binary relevance for precision/recall
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub total_files: usize,
    pub total_queries: usize,
    pub languages: Vec<String>,
    pub creation_date: String,
    pub version: String,
}

pub struct TestDatasetGenerator {
    base_path: PathBuf,
    ground_truth: GroundTruth,
    files_created: Vec<String>,
}

impl TestDatasetGenerator {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            ground_truth: GroundTruth {
                queries: Vec::new(),
                relevance_mappings: HashMap::new(),
                metadata: DatasetMetadata {
                    total_files: 0,
                    total_queries: 0,
                    languages: vec![
                        "rust".to_string(),
                        "typescript".to_string(),
                        "python".to_string(),
                        "go".to_string(),
                        "java".to_string(),
                    ],
                    creation_date: chrono::Utc::now().to_rfc3339(),
                    version: "1.0.0".to_string(),
                },
            },
            files_created: Vec::new(),
        }
    }
    
    pub fn create_directory_structure(&self) -> Result<()> {
        let dirs = vec![
            "rust/auth",
            "rust/database",
            "rust/api",
            "rust/utils",
            "rust/tests",
            "typescript/components",
            "typescript/services",
            "typescript/utils",
            "typescript/tests",
            "python/models",
            "python/services",
            "python/ml",
            "python/tests",
            "go/handlers",
            "go/services",
            "go/utils",
            "java/controllers",
            "java/services",
            "java/models",
        ];
        
        for dir in dirs {
            fs::create_dir_all(self.base_path.join(dir))?;
        }
        
        Ok(())
    }
    
    pub fn write_file(&mut self, relative_path: &str, content: &str) -> Result<()> {
        let full_path = self.base_path.join(relative_path);
        fs::write(&full_path, content)?;
        self.files_created.push(relative_path.to_string());
        self.ground_truth.metadata.total_files += 1;
        Ok(())
    }
    
    pub fn add_query(&mut self, query: TestQuery) {
        // Add relevance mappings
        for expected in &query.expected_results {
            let score = RelevanceScore {
                query_id: query.id.clone(),
                file_path: expected.file_path.clone(),
                relevance: expected.relevance,
                is_relevant: expected.relevance >= 0.5,
            };
            
            self.ground_truth.relevance_mappings
                .entry(query.id.clone())
                .or_insert_with(Vec::new)
                .push(score);
        }
        
        self.ground_truth.queries.push(query);
        self.ground_truth.metadata.total_queries += 1;
    }
    
    pub fn save_ground_truth(&self) -> Result<()> {
        let gt_path = self.base_path.join("ground_truth.json");
        let json = serde_json::to_string_pretty(&self.ground_truth)?;
        fs::write(gt_path, json)?;
        Ok(())
    }
    
    pub fn get_ground_truth(&self) -> &GroundTruth {
        &self.ground_truth
    }
}
