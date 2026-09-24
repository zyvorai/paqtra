fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "grpc")]
    {
        tonic_prost_build::compile_protos("proto/flow.proto")?;
    }
    Ok(())
}
