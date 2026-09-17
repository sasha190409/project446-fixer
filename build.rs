fn main() {
    #[cfg(windows)]
    {
        use std::path::PathBuf;

        let mut resource = winres::WindowsResource::new();

        let icon_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("project446.ico");

        println!("cargo:rerun-if-changed={}", icon_path.display());

        resource.set_icon(icon_path.to_string_lossy().as_ref());

        resource
            .compile()
            .expect("failed to compile Windows resources");
    }
}