use std::io::Result;
fn main() -> Result<()> {
    prost_build::compile_protos(&["proto/model.proto", "proto/message.proto"], &["proto/"])?;
    Ok(())
}