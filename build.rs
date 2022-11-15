fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto/raft_rpc.proto");

    let config = prost_build::Config::new();

    tonic_build::configure()
        .out_dir("src/rpc")
        .compile_with_config(config, &["proto/raft_rpc.proto"], &["proto"])?;

    Ok(())
}
