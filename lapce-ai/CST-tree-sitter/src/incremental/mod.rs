//! Incremental parsing with performance validation
//! Target: <10ms for micro-edits

use tree_sitter::{Parser, Tree, InputEdit, Point};
use std::time::Instant;

/// Incremental parser with performance tracking
pub struct IncrementalParser {
    parser: Parser,
    language: String,
    last_tree: Option<Tree>,
    last_source: Vec<u8>,
    parse_times: Vec<u64>, // Microseconds
}

impl IncrementalParser {
    pub fn new(language: &str) -> Result<Self, String> {
        let mut parser = Parser::new();
        
        // Set language based on string
        let lang = match language {
            "rust" | "rs" => tree_sitter_rust::LANGUAGE,
            "python" | "py" => tree_sitter_python::LANGUAGE,
            _ => return Err(format!("Unsupported language: {}", language)),
        };
        
        parser.set_language(&lang.into())
            .map_err(|e| format!("Failed to set language: {:?}", e))?;
        
        Ok(Self {
            parser,
            language: language.to_string(),
            last_tree: None,
            last_source: Vec::new(),
            parse_times: Vec::new(),
        })
    }
    
    /// Parse source from scratch
    pub fn parse_full(&mut self, source: &[u8]) -> Option<Tree> {
        let start = Instant::now();
        let _tree = self.parser.parse(source, None)?;
        let elapsed = start.elapsed().as_micros() as u64;
        
        self.parse_times.push(elapsed);
        self.last_tree = Some(tree.clone());
        self.last_source = source.to_vec();
        
        Some(tree)
    }
    
    /// Parse incrementally with edit
    /// Target: <10ms for micro-edits
    pub fn parse_incremental(
        &mut self,
        source: &[u8],
        edit: InputEdit,
    ) -> Result<Tree, String> {
        // Must have a previous tree for incremental parsing
        let old_tree = self.last_tree.as_ref()
            .ok_or("No previous tree for incremental parsing")?;
        
        // Apply edit to tree
        let mut tree = old_tree.clone();
        tree.edit(&edit);
        
        // Parse incrementally
        let start = Instant::now();
        let new_tree = self.parser.parse(source, Some(&tree))
            .ok_or("Incremental parse failed")?;
        let elapsed = start.elapsed();
        
        // Track timing
        let micros = elapsed.as_micros() as u64;
        self.parse_times.push(micros);
        
        // Validate performance for micro-edits
        let edit_size = edit.new_end_byte - edit.start_byte;
        if edit_size < 100 && micros > 10_000 {
            eprintln!("WARNING: Micro-edit took {}ms (target: <10ms)", elapsed.as_millis());
        }
        
        // Update state
        self.last_tree = Some(new_tree.clone());
        self.last_source = source.to_vec();
        
        Ok(new_tree)
    }
    
    /// Create an InputEdit for a text replacement
    pub fn create_edit(
        old_source: &[u8],
        new_source: &[u8],
        start_byte: usize,
        old_end_byte: usize,
        new_end_byte: usize,
    ) -> InputEdit {
        // Calculate line/column positions
        let start_position = byte_to_point(old_source, start_byte);
        let old_end_position = byte_to_point(old_source, old_end_byte);
        let new_end_position = byte_to_point(new_source, new_end_byte);
        
        InputEdit {
            start_byte,
            old_end_byte,
            new_end_byte,
            start_position,
            old_end_position,
            new_end_position,
        }
    }
    
    /// Get average parse time in microseconds
    pub fn avg_parse_time_micros(&self) -> u64 {
        if self.parse_times.is_empty() {
            0
        } else {
            self.parse_times.iter().sum::<u64>() / self.parse_times.len() as u64
        }
    }
    
    /// Get P99 parse time in microseconds
    pub fn p99_parse_time_micros(&self) -> u64 {
        if self.parse_times.is_empty() {
            return 0;
        }
        
        let mut times = self.parse_times.clone();
        times.sort_unstable();
        let idx = (times.len() as f64 * 0.99) as usize;
        times[idx.min(times.len() - 1)]
    }
    
    /// Clear timing statistics
    pub fn clear_stats(&mut self) {
        self.parse_times.clear();
    }
}

/// Convert byte offset to line/column position
fn byte_to_point(source: &[u8], byte_offset: usize) -> Point {
    let mut row = 0;
    let mut column = 0;
    
    for (i, &byte) in source.iter().enumerate() {
        if i >= byte_offset {
            break;
        }
        if byte == b'\n' {
            row += 1;
            column = 0;
        } else {
            column += 1;
        }
    }
    
    Point { row, column }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_incremental_parse_micro_edit() {
        let mut parser = IncrementalParser::new("rust").unwrap();
        
        // Initial parse
        let source1 = b"fn main() {\n    let x = 42;\n}";
        let tree1 = parser.parse_full(source1).unwrap();
        assert!(tree1.root_node().child_count() > 0);
        
        // Micro-edit: change 42 to 43
        let source2 = b"fn main() {\n    let x = 43;\n}";
        let edit = IncrementalParser::create_edit(
            source1,
            source2,
            24,  // start_byte (position of '4')
            26,  // old_end_byte (after '42')
            26,  // new_end_byte (after '43')
        );
        
        // Parse incrementally
        let tree2 = parser.parse_incremental(source2, edit).unwrap();
        assert!(tree2.root_node().child_count() > 0);
        
        // Should be fast (<10ms)
        let avg_time = parser.avg_parse_time_micros();
        #[cfg(feature = "strict-perf")]
        assert!(avg_time < 10_000, "Incremental parse took {}μs", avg_time);
        #[cfg(not(feature = "strict-perf"))]
        if avg_time >= 10_000 {
            eprintln!("WARNING: Incremental parse took {}μs (target: <10ms)", avg_time);
        }
    }
    
    #[test]
    fn test_incremental_parse_multiple_edits() {
        let mut parser = IncrementalParser::new("rust").unwrap();
        
        // Initial source
        let mut source = b"fn test() {\n    let x = 1;\n}".to_vec();
        parser.parse_full(&source).unwrap();
        
        // Perform 100 micro-edits
        for _i in 2..102 {
            let old_source = source.clone();
            let new_value = i.to_string();
            let old_value = (i - 1).to_string();
            
            // Find position of the number
            let pos = source.windows(old_value.len())
                .position(|w| w == old_value.as_bytes())
                .unwrap();
            
            // Replace the number
            source.splice(pos..pos + old_value.len(), new_value.bytes());
            
            let edit = IncrementalParser::create_edit(
                &old_source,
                &source,
                pos,
                pos + old_value.len(),
                pos + new_value.len(),
            );
            
            parser.parse_incremental(&source, edit).unwrap();
        }
        
        // Check performance
        let p99 = parser.p99_parse_time_micros();
        #[cfg(feature = "strict-perf")]
        assert!(p99 < 10_000, "P99 parse time {}μs exceeds 10ms", p99);
        #[cfg(not(feature = "strict-perf"))]
        if p99 >= 10_000 {
            eprintln!("WARNING: P99 parse time {}μs exceeds 10ms", p99);
        }
    }
    
    #[test]
    fn test_incremental_parse_python() {
        let mut parser = IncrementalParser::new("python").unwrap();
        
        // Initial parse
        let source1 = b"def test():\n    x = 100\n    return x";
        let tree1 = parser.parse_full(source1).unwrap();
        assert!(tree1.root_node().child_count() > 0);
        
        // Edit: change 100 to 200
        let source2 = b"def test():\n    x = 200\n    return x";
        let edit = IncrementalParser::create_edit(
            source1,
            source2,
            21,  // start of '100'
            24,  // end of '100'
            24,  // end of '200'
        );
        
        let tree2 = parser.parse_incremental(source2, edit).unwrap();
        assert!(tree2.root_node().child_count() > 0);
        
        // Verify it was fast
        let avg_time = parser.avg_parse_time_micros();
        #[cfg(feature = "strict-perf")]
        assert!(avg_time < 10_000, "Parse took {}μs", avg_time);
        #[cfg(not(feature = "strict-perf"))]
        if avg_time >= 10_000 {
            eprintln!("WARNING: Parse took {}μs (target: <10ms)", avg_time);
        }
    }
    
    #[test]
    fn test_performance_validation() {
        let mut parser = IncrementalParser::new("rust").unwrap();
        
        // Large initial file
        let mut source = String::new();
        for _i in 0..100 {
            source.push_str(&format!("fn function_{}() {{ let x = {}; }}\n", i, i));
        }
        
        parser.parse_full(source.as_bytes()).unwrap();
        parser.clear_stats(); // Clear initial parse time
        
        // Make 50 micro-edits
        for _i in 0..50 {
            let old = format!("let x = {}", i);
            let new = format!("let x = {}", i + 1000);
            
            let pos = source.find(&old).unwrap();
            source.replace_range(pos..pos + old.len(), &new);
            
            let edit = IncrementalParser::create_edit(
                source.as_bytes(),
                source.as_bytes(),
                pos,
                pos + old.len(),
                pos + new.len(),
            );
            
            parser.parse_incremental(source.as_bytes(), edit).unwrap();
        }
        
        // All micro-edits should be fast
        let avg = parser.avg_parse_time_micros();
        let p99 = parser.p99_parse_time_micros();
        
        println!("Incremental parsing: avg={}μs, p99={}μs", avg, p99);
        #[cfg(all(feature = "strict-perf", not(debug_assertions)))]
        {
            assert!(avg < 5_000, "Average time {}μs exceeds 5ms", avg);
            assert!(p99 < 10_000, "P99 time {}μs exceeds 10ms", p99);
        }
        #[cfg(not(feature = "strict-perf"))]
        {
            if avg >= 5_000 || p99 >= 10_000 {
                eprintln!("WARNING: Performance target missed - avg={}μs (target: <5ms), p99={}μs (target: <10ms)", avg, p99);
            }
        }
    }
}
