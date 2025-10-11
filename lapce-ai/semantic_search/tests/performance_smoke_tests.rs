// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors
// Performance smoke tests - validate parse/transform times on real files

#[cfg(feature = "cst_ts")]
use lancedb::processors::cst_to_ast_pipeline::CstToAstPipeline;
use std::io::Write;
use tempfile::NamedTempFile;
use std::time::Instant;

#[cfg(feature = "cst_ts")]
#[tokio::test]
async fn test_rust_parse_performance_small() {
    let code = r#"
fn add(x: i32, y: i32) -> i32 {
    x + y
}

fn main() {
    let result = add(5, 3);
    println!("Result: {}", result);
}
"#;
    
    let mut temp_file = NamedTempFile::with_suffix(".rs").unwrap();
    temp_file.write_all(code.as_bytes()).unwrap();
    let path = temp_file.path();
    
    let pipeline = CstToAstPipeline::new();
    let start = Instant::now();
    let result = pipeline.process_file(path).await.unwrap();
    let duration = start.elapsed();
    
    // Small files should parse in <10ms
    assert!(duration.as_millis() < 10, "Parse took {}ms, expected <10ms", duration.as_millis());
    assert!(result.parse_time_ms < 10.0, "Parse time {}ms exceeds 10ms", result.parse_time_ms);
    assert!(result.transform_time_ms < 5.0, "Transform time {}ms exceeds 5ms", result.transform_time_ms);
}

#[cfg(feature = "cst_ts")]
#[tokio::test]
async fn test_rust_parse_performance_medium() {
    // ~100 lines of code
    let code = r#"
use std::collections::HashMap;

pub struct Config {
    settings: HashMap<String, String>,
    cache: HashMap<String, Vec<u8>>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            settings: HashMap::new(),
            cache: HashMap::new(),
        }
    }
    
    pub fn get(&self, key: &str) -> Option<&String> {
        self.settings.get(key)
    }
    
    pub fn set(&mut self, key: String, value: String) {
        self.settings.insert(key, value);
    }
    
    pub fn get_cache(&self, key: &str) -> Option<&Vec<u8>> {
        self.cache.get(key)
    }
    
    pub fn set_cache(&mut self, key: String, value: Vec<u8>) {
        self.cache.insert(key, value);
    }
}

pub fn process_data(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    for byte in data {
        result.push(byte.wrapping_add(1));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config() {
        let mut config = Config::new();
        config.set("key".to_string(), "value".to_string());
        assert_eq!(config.get("key"), Some(&"value".to_string()));
    }
}
"#;
    
    let mut temp_file = NamedTempFile::with_suffix(".rs").unwrap();
    temp_file.write_all(code.as_bytes()).unwrap();
    let path = temp_file.path();
    
    let pipeline = CstToAstPipeline::new();
    let start = Instant::now();
    let result = pipeline.process_file(path).await.unwrap();
    let duration = start.elapsed();
    
    // Medium files should parse in <50ms
    assert!(duration.as_millis() < 50, "Parse took {}ms, expected <50ms", duration.as_millis());
    assert!(result.parse_time_ms < 50.0, "Parse time {}ms exceeds 50ms", result.parse_time_ms);
}

#[cfg(feature = "cst_ts")]
#[tokio::test]
async fn test_javascript_parse_performance() {
    let code = r#"
class DataProcessor {
    constructor() {
        this.cache = new Map();
        this.results = [];
    }
    
    async processItem(item) {
        if (this.cache.has(item.id)) {
            return this.cache.get(item.id);
        }
        
        const result = await this.transform(item);
        this.cache.set(item.id, result);
        this.results.push(result);
        return result;
    }
    
    transform(item) {
        return new Promise((resolve) => {
            setTimeout(() => {
                resolve({ ...item, processed: true });
            }, 10);
        });
    }
}

async function main() {
    const processor = new DataProcessor();
    const items = [
        { id: 1, data: 'test1' },
        { id: 2, data: 'test2' },
    ];
    
    for (const item of items) {
        await processor.processItem(item);
    }
    
    console.log('Processed:', processor.results.length);
}

main();
"#;
    
    let mut temp_file = NamedTempFile::with_suffix(".js").unwrap();
    temp_file.write_all(code.as_bytes()).unwrap();
    let path = temp_file.path();
    
    let pipeline = CstToAstPipeline::new();
    let start = Instant::now();
    let result = pipeline.process_file(path).await.unwrap();
    let duration = start.elapsed();
    
    assert!(duration.as_millis() < 50, "Parse took {}ms, expected <50ms", duration.as_millis());
    assert_eq!(result.language, "javascript");
}

#[cfg(feature = "cst_ts")]
#[tokio::test]
async fn test_python_parse_performance() {
    let code = r#"
import asyncio
from typing import Dict, List, Optional

class DataProcessor:
    def __init__(self):
        self.cache: Dict[int, dict] = {}
        self.results: List[dict] = []
    
    async def process_item(self, item: dict) -> dict:
        if item['id'] in self.cache:
            return self.cache[item['id']]
        
        result = await self.transform(item)
        self.cache[item['id']] = result
        self.results.append(result)
        return result
    
    async def transform(self, item: dict) -> dict:
        await asyncio.sleep(0.01)
        return {**item, 'processed': True}

async def main():
    processor = DataProcessor()
    items = [
        {'id': 1, 'data': 'test1'},
        {'id': 2, 'data': 'test2'},
    ]
    
    for item in items:
        await processor.process_item(item)
    
    print(f'Processed: {len(processor.results)}')

if __name__ == '__main__':
    asyncio.run(main())
"#;
    
    let mut temp_file = NamedTempFile::with_suffix(".py").unwrap();
    temp_file.write_all(code.as_bytes()).unwrap();
    let path = temp_file.path();
    
    let pipeline = CstToAstPipeline::new();
    let start = Instant::now();
    let result = pipeline.process_file(path).await.unwrap();
    let duration = start.elapsed();
    
    assert!(duration.as_millis() < 50, "Parse took {}ms, expected <50ms", duration.as_millis());
    assert_eq!(result.language, "python");
}

#[cfg(feature = "cst_ts")]
#[tokio::test]
async fn test_concurrent_parsing_performance() {
    let codes = vec![
        ("rust", ".rs", "fn test() { println!(\"test\"); }"),
        ("javascript", ".js", "function test() { console.log('test'); }"),
        ("python", ".py", "def test():\n    print('test')"),
    ];
    
    let pipeline = CstToAstPipeline::new();
    let start = Instant::now();
    
    let mut handles = vec![];
    for (_, ext, code) in codes {
        let pipeline = CstToAstPipeline::new();
        let handle = tokio::spawn(async move {
            let mut temp_file = NamedTempFile::with_suffix(ext).unwrap();
            temp_file.write_all(code.as_bytes()).unwrap();
            let path = temp_file.path().to_path_buf();
            pipeline.process_file(&path).await
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await.unwrap().unwrap();
    }
    
    let duration = start.elapsed();
    
    // Concurrent parsing should be faster than sequential
    // 3 files should take <100ms total
    assert!(duration.as_millis() < 100, "Concurrent parse took {}ms, expected <100ms", duration.as_millis());
}

#[cfg(feature = "cst_ts")]
#[tokio::test]
async fn test_parse_real_world_rust_file() {
    // Use a real file from the codebase if it exists
    let pipeline = CstToAstPipeline::new();
    let test_file = std::path::Path::new("src/processors/cst_to_ast_pipeline.rs");
    
    if test_file.exists() {
        let start = Instant::now();
        let result = pipeline.process_file(test_file).await.unwrap();
        let duration = start.elapsed();
        
        println!("Real file parse: {}ms (parse: {:.2}ms, transform: {:.2}ms)",
                 duration.as_millis(), result.parse_time_ms, result.transform_time_ms);
        
        // Real files up to ~1000 lines should parse in <500ms
        assert!(duration.as_millis() < 500, "Real file parse took {}ms, expected <500ms", duration.as_millis());
    }
}
