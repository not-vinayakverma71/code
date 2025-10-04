//! PARSER POOL - EFFICIENT PARSER REUSE FOR PERFORMANCE

use crate::native_parser_manager::FileType;
use tree_sitter::Parser;
use dashmap::DashMap;
use std::sync::Arc;

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
            FileType::C => parser.set_language(&tree_sitter_c::LANGUAGE.into()),
            FileType::Cpp => parser.set_language(&tree_sitter_cpp::LANGUAGE.into()),
            FileType::CSharp => parser.set_language(&tree_sitter_c_sharp::LANGUAGE.into()),
            FileType::Ruby => parser.set_language(&tree_sitter_ruby::LANGUAGE.into()),
            FileType::Php => parser.set_language(&tree_sitter_php::LANGUAGE_PHP.into()),
            FileType::Lua => parser.set_language(&tree_sitter_lua::LANGUAGE.into()),
            FileType::Bash => parser.set_language(&tree_sitter_bash::LANGUAGE.into()),
            FileType::Css => parser.set_language(&tree_sitter_css::LANGUAGE.into()),
            FileType::Json => parser.set_language(&tree_sitter_json::LANGUAGE.into()),
            FileType::Toml => return Err("TOML parser version conflict".into()),
            FileType::Dockerfile => return Err("Dockerfile parser version conflict".into()),
            FileType::Yaml => return Err("YAML parser version conflict".into()),
            FileType::Swift => parser.set_language(&tree_sitter_swift::LANGUAGE.into()),
            FileType::Kotlin => return Err("Kotlin parser not available".into()),
            FileType::Scala => parser.set_language(&tree_sitter_scala::LANGUAGE.into()),
            FileType::Haskell => return Err("Haskell parser not available".into()),
            FileType::Markdown => return Err("Markdown parser version conflict".into()),
            FileType::Elixir => parser.set_language(&tree_sitter_elixir::LANGUAGE.into()),
            FileType::Html => parser.set_language(&tree_sitter_html::LANGUAGE.into()),
            FileType::Ocaml => parser.set_language(&tree_sitter_ocaml::LANGUAGE_OCAML.into()),
            FileType::Elm => parser.set_language(&tree_sitter_elm::LANGUAGE.into()),
            FileType::Svelte => return Err("Svelte parser version conflict".into()),
            FileType::Erlang => return Err("Erlang parser not available".into()),
            FileType::Nim => return Err("Nim parser not available".into()),
            _ => return Err("Language not supported".into()),
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
