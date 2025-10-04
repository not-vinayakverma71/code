fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check if proto files exist before trying to compile
    let proto_files = ["proto/ipc.proto", "proto/service.proto"];
    let mut existing_protos = Vec::new();
    
    for proto in &proto_files {
        if std::path::Path::new(proto).exists() {
            existing_protos.push(*proto);
        }
    }
    
    if !existing_protos.is_empty() {
        tonic_build::configure()
            .build_server(true)
            .build_client(true)
            .compile(&existing_protos, &["proto"])?;
    }
    
    // Tree-sitter parsers are already compiled by their respective crates
    // No additional build steps needed - the crates handle C compilation
    
    Ok(())
}
