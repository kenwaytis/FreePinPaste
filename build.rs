fn main() {
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rerun-if-changed=packaging/windows/app.rc");
        println!("cargo:rerun-if-changed=assets/freepinpaste.ico");
        embed_resource::compile("packaging/windows/app.rc", embed_resource::NONE)
            .manifest_required()
            .expect("Windows application resources");
    }
}
