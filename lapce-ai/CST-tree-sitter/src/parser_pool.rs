//! PARSER POOL - EFFICIENT PARSER REUSE FOR PERFORMANCE

use tree_sitter::Parser;
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    Rust,
    JavaScript,
    TypeScript,
    Python,
    Go,
    Cpp,
    Java,
    Other,
}

pub struct ParserPool {
    pools: DashMap<FileType, Vec<Parser>>,
    max_per_type: usize,
}

impl ParserPool {
    pub fn new(max_per_type: usize) -> Self {
        Self {
            pools: DashMap::new(),
            max_per_type,
        }
    }
    
    pub fn acquire(&self, file_type: FileType) -> Result<PooledParser<'_>, Box<dyn std::error::Error>> {
        let mut pool = self.pools.entry(file_type).or_insert_with(Vec::new);
        
        let parser = if let Some(parser) = pool.pop() {
            parser
        } else {
            Self::create_parser(file_type)?
        };
        
        Ok(PooledParser {
            parser: Some(parser),
            file_type,
            pool: self,
        })
    }
    
    pub fn release(&self, file_type: FileType, parser: Parser) {
        if let Some(mut pool) = self.pools.get_mut(&file_type) {
            if pool.len() < self.max_per_type {
                pool.push(parser);
            }
        }
    }
    
    fn create_parser(file_type: FileType) -> Result<Parser, Box<dyn std::error::Error>> {
        let mut parser = Parser::new();
        let result = match file_type {
            FileType::Rust => parser.set_language(&tree_sitter_rust::LANGUAGE.into()),
            FileType::JavaScript => parser.set_language(&tree_sitter_javascript::language()),
            FileType::TypeScript => parser.set_language(&tree_sitter_typescript::language_typescript()),
            FileType::Python => parser.set_language(&tree_sitter_python::LANGUAGE.into()),
            FileType::Go => parser.set_language(&tree_sitter_go::LANGUAGE.into()),
            FileType::Java => parser.set_language(&tree_sitter_java::LANGUAGE.into()),
            FileType::Cpp => parser.set_language(&tree_sitter_cpp::LANGUAGE.into()),
            FileType::Other => return Err("Unsupported language".into()),
        };
        
        result?;
        Ok(parser)
    }
}

pub struct PooledParser<'a> {
    parser: Option<Parser>,
    file_type: FileType,
    pool: &'a ParserPool,
}

impl<'a> PooledParser<'a> {
    pub fn get_mut(&mut self) -> &mut Parser {
        self.parser.as_mut().unwrap()
    }
}

impl<'a> Drop for PooledParser<'a> {
    fn drop(&mut self) {
        if let Some(parser) = self.parser.take() {
            self.pool.release(self.file_type, parser);
        }
    }
}
