fn main() {
  // Un único manifiesto (controles comunes v6) para la app y para los ejecutables de pruebas.
  // Sin él, las pruebas fallan con STATUS_ENTRYPOINT_NOT_FOUND.
  let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("test.manifest");
  println!("cargo:rerun-if-changed=test.manifest");
  println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
  println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
  let attrs = tauri_build::Attributes::new()
    .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
  tauri_build::try_build(attrs).expect("error al preparar la app");
}
