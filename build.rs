fn main() {
    #[cfg(windows)]
    {
        use std::path::PathBuf;

        let mut resource = winres::WindowsResource::new();

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let icon_path = manifest_dir.join("assets").join("project446.ico");

        println!("cargo:rerun-if-changed={}", icon_path.display());

        resource.set_icon(icon_path.to_string_lossy().as_ref());
        resource.set("CompanyName", "Project446");
        resource.set("FileDescription", "CS:GO Legacy Fixer");
        resource.set("ProductName", "CS:GO Legacy Fixer");
        resource.set("LegalCopyright", "Copyright (C) Project446");
        resource.set("OriginalFilename", "csgo-legacy-fixer.exe");
        resource.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        resource.set("FileVersion", env!("CARGO_PKG_VERSION"));
        resource.set("InternalName", "csgo-legacy-fixer");

        // SxS-манифест. version берётся из Cargo.toml, чтобы свойства файла
        // и assemblyIdentity не разъезжались.
        let sxs_version = format!("{}.0", env!("CARGO_PKG_VERSION"));
        let manifest = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity
      type="win32"
      name="Project446.CSGOLegacyFixer"
      version="{sxs_version}"
      processorArchitecture="*"/>
  <description>CS:GO Legacy Fixer</description>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="requireAdministrator" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}}"/>
    </application>
  </compatibility>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
          type="win32"
          name="Microsoft.Windows.Common-Controls"
          version="6.0.0.0"
          processorArchitecture="*"
          publicKeyToken="6595b64144ccf1df"
          language="*"/>
    </dependentAssembly>
  </dependency>
</assembly>"#
        );
        resource.set_manifest(&manifest);

        resource
            .compile()
            .expect("failed to compile Windows resources");
    }
}
