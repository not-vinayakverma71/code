// Complete list of tree-sitter language support
// This file contains all language configurations for the CST pipeline

use std::collections::HashMap;

pub struct LanguageConfig {
    pub name: &'static str,
    pub extensions: Vec<&'static str>,
    pub tree_sitter_name: &'static str,
    pub supported: bool,
}

impl LanguageConfig {
    pub fn new(name: &'static str, extensions: Vec<&'static str>, tree_sitter_name: &'static str, supported: bool) -> Self {
        Self {
            name,
            extensions,
            tree_sitter_name,
            supported,
        }
    }
}

pub fn get_all_languages() -> Vec<LanguageConfig> {
    vec![
        // Currently supported languages (9)
        LanguageConfig::new("rust", vec!["rs"], "tree-sitter-rust", true),
        LanguageConfig::new("javascript", vec!["js", "jsx", "mjs", "cjs"], "tree-sitter-javascript", true),
        LanguageConfig::new("typescript", vec!["ts", "tsx", "mts", "cts"], "tree-sitter-typescript", true),
        LanguageConfig::new("python", vec!["py", "pyi"], "tree-sitter-python", true),
        LanguageConfig::new("go", vec!["go"], "tree-sitter-go", true),
        LanguageConfig::new("java", vec!["java"], "tree-sitter-java", true),
        LanguageConfig::new("cpp", vec!["cpp", "cc", "cxx", "hpp", "hxx", "h++"], "tree-sitter-cpp", true),
        LanguageConfig::new("c", vec!["c", "h"], "tree-sitter-c", true),
        LanguageConfig::new("c_sharp", vec!["cs", "csx"], "tree-sitter-c-sharp", true),
        
        // Additional native tree-sitter languages to add (58+)
        LanguageConfig::new("ruby", vec!["rb", "gemspec", "podspec", "thor", "irb"], "tree-sitter-ruby", false),
        LanguageConfig::new("php", vec!["php", "php3", "php4", "php5", "phps", "phtml"], "tree-sitter-php", false),
        LanguageConfig::new("swift", vec!["swift"], "tree-sitter-swift", false),
        LanguageConfig::new("kotlin", vec!["kt", "kts"], "tree-sitter-kotlin", false),
        LanguageConfig::new("scala", vec!["scala", "sc"], "tree-sitter-scala", false),
        LanguageConfig::new("haskell", vec!["hs", "lhs"], "tree-sitter-haskell", false),
        LanguageConfig::new("elixir", vec!["ex", "exs"], "tree-sitter-elixir", false),
        LanguageConfig::new("lua", vec!["lua"], "tree-sitter-lua", false),
        LanguageConfig::new("bash", vec!["sh", "bash", "zsh", "fish"], "tree-sitter-bash", false),
        LanguageConfig::new("html", vec!["html", "htm", "xhtml"], "tree-sitter-html", false),
        LanguageConfig::new("css", vec!["css"], "tree-sitter-css", false),
        LanguageConfig::new("json", vec!["json"], "tree-sitter-json", false),
        LanguageConfig::new("yaml", vec!["yaml", "yml"], "tree-sitter-yaml", false),
        LanguageConfig::new("toml", vec!["toml"], "tree-sitter-toml", false),
        LanguageConfig::new("markdown", vec!["md", "markdown"], "tree-sitter-markdown", false),
        LanguageConfig::new("sql", vec!["sql"], "tree-sitter-sql", false),
        LanguageConfig::new("dockerfile", vec!["dockerfile", "Dockerfile"], "tree-sitter-dockerfile", false),
        LanguageConfig::new("cmake", vec!["cmake", "CMakeLists.txt"], "tree-sitter-cmake", false),
        LanguageConfig::new("make", vec!["makefile", "Makefile", "mk", "mak"], "tree-sitter-make", false),
        LanguageConfig::new("vim", vec!["vim", "vimrc"], "tree-sitter-vim", false),
        LanguageConfig::new("latex", vec!["tex", "sty", "cls"], "tree-sitter-latex", false),
        LanguageConfig::new("ocaml", vec!["ml", "mli"], "tree-sitter-ocaml", false),
        LanguageConfig::new("erlang", vec!["erl", "hrl"], "tree-sitter-erlang", false),
        LanguageConfig::new("julia", vec!["jl"], "tree-sitter-julia", false),
        LanguageConfig::new("r", vec!["r", "R"], "tree-sitter-r", false),
        LanguageConfig::new("dart", vec!["dart"], "tree-sitter-dart", false),
        LanguageConfig::new("zig", vec!["zig"], "tree-sitter-zig", false),
        LanguageConfig::new("nim", vec!["nim", "nims"], "tree-sitter-nim", false),
        LanguageConfig::new("nix", vec!["nix"], "tree-sitter-nix", false),
        LanguageConfig::new("perl", vec!["pl", "pm", "pod"], "tree-sitter-perl", false),
        LanguageConfig::new("clojure", vec!["clj", "cljs", "cljc", "edn"], "tree-sitter-clojure", false),
        LanguageConfig::new("elm", vec!["elm"], "tree-sitter-elm", false),
        LanguageConfig::new("fortran", vec!["f", "for", "f90", "f95", "f03"], "tree-sitter-fortran", false),
        LanguageConfig::new("ada", vec!["ada", "adb", "ads"], "tree-sitter-ada", false),
        LanguageConfig::new("pascal", vec!["pas", "pp"], "tree-sitter-pascal", false),
        LanguageConfig::new("d", vec!["d", "di"], "tree-sitter-d", false),
        LanguageConfig::new("verilog", vec!["v", "vh", "sv", "svh"], "tree-sitter-verilog", false),
        LanguageConfig::new("vhdl", vec!["vhd", "vhdl"], "tree-sitter-vhdl", false),
        LanguageConfig::new("graphql", vec!["graphql", "gql"], "tree-sitter-graphql", false),
        LanguageConfig::new("proto", vec!["proto"], "tree-sitter-proto", false),
        LanguageConfig::new("thrift", vec!["thrift"], "tree-sitter-thrift", false),
        LanguageConfig::new("cuda", vec!["cu", "cuh"], "tree-sitter-cuda", false),
        LanguageConfig::new("glsl", vec!["glsl", "vert", "frag", "geom"], "tree-sitter-glsl", false),
        LanguageConfig::new("wgsl", vec!["wgsl"], "tree-sitter-wgsl", false),
        LanguageConfig::new("hlsl", vec!["hlsl", "fx", "fxh", "hlsli"], "tree-sitter-hlsl", false),
        LanguageConfig::new("solidity", vec!["sol"], "tree-sitter-solidity", false),
        LanguageConfig::new("move", vec!["move"], "tree-sitter-move", false),
        LanguageConfig::new("cairo", vec!["cairo"], "tree-sitter-cairo", false),
        LanguageConfig::new("fennel", vec!["fnl"], "tree-sitter-fennel", false),
        LanguageConfig::new("fish", vec!["fish"], "tree-sitter-fish", false),
        LanguageConfig::new("gleam", vec!["gleam"], "tree-sitter-gleam", false),
        LanguageConfig::new("hack", vec!["hack", "hck"], "tree-sitter-hack", false),
        LanguageConfig::new("hcl", vec!["hcl", "tf", "tfvars"], "tree-sitter-hcl", false),
        LanguageConfig::new("jsonnet", vec!["jsonnet", "libsonnet"], "tree-sitter-jsonnet", false),
        LanguageConfig::new("just", vec!["just", "justfile"], "tree-sitter-just", false),
        LanguageConfig::new("kdl", vec!["kdl"], "tree-sitter-kdl", false),
        LanguageConfig::new("llvm", vec!["ll"], "tree-sitter-llvm", false),
        LanguageConfig::new("meson", vec!["meson", "meson.build"], "tree-sitter-meson", false),
        LanguageConfig::new("ninja", vec!["ninja"], "tree-sitter-ninja", false),
        LanguageConfig::new("nu", vec!["nu"], "tree-sitter-nu", false),
        LanguageConfig::new("objc", vec!["m", "mm"], "tree-sitter-objc", false),
        LanguageConfig::new("odin", vec!["odin"], "tree-sitter-odin", false),
        LanguageConfig::new("prisma", vec!["prisma"], "tree-sitter-prisma", false),
        LanguageConfig::new("puppet", vec!["pp"], "tree-sitter-puppet", false),
        LanguageConfig::new("qml", vec!["qml"], "tree-sitter-qml", false),
        LanguageConfig::new("racket", vec!["rkt"], "tree-sitter-racket", false),
        LanguageConfig::new("rescript", vec!["res", "resi"], "tree-sitter-rescript", false),
        LanguageConfig::new("scheme", vec!["scm", "ss"], "tree-sitter-scheme", false),
        LanguageConfig::new("smithy", vec!["smithy"], "tree-sitter-smithy", false),
        LanguageConfig::new("svelte", vec!["svelte"], "tree-sitter-svelte", false),
        LanguageConfig::new("tablegen", vec!["td"], "tree-sitter-tablegen", false),
        LanguageConfig::new("teal", vec!["tl"], "tree-sitter-teal", false),
        LanguageConfig::new("terraform", vec!["tf", "tfvars"], "tree-sitter-terraform", false),
        LanguageConfig::new("v", vec!["v", "vsh"], "tree-sitter-v", false),
        LanguageConfig::new("vue", vec!["vue"], "tree-sitter-vue", false),
        LanguageConfig::new("wast", vec!["wast", "wat"], "tree-sitter-wast", false),
        LanguageConfig::new("xml", vec!["xml", "xsd", "xsl", "xslt", "svg"], "tree-sitter-xml", false),
        LanguageConfig::new("zig", vec!["zig"], "tree-sitter-zig", false),
    ]
}

pub fn get_extension_map() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    for lang in get_all_languages() {
        for ext in lang.extensions {
            map.insert(ext, lang.name);
        }
    }
    map
}

pub fn get_language_stats() -> (usize, usize) {
    let langs = get_all_languages();
    let supported = langs.iter().filter(|l| l.supported).count();
    let total = langs.len();
    (supported, total)
}
