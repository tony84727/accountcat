fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=migrations");
    tonic_build::compile_protos("../proto/user.proto")?;
    tonic_build::compile_protos("../proto/todolist.proto")?;
    tonic_build::compile_protos("../proto/accounting.proto")?;
    tonic_build::compile_protos("../proto/instance_setting.proto")?;
    tonic_build::compile_protos("../proto/testing.proto")?;
    Ok(())
}
