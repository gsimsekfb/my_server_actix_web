fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe { std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path()?); }

    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("twn_descriptor.bin"))
        .compile_protos(&["proto/twn.proto"], &["proto"])?;

    Ok(())
}
