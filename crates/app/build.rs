use std::path::PathBuf;

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set"),
    );
    let icon_path = manifest_dir.join("assets").join("lanscanner.ico");

    println!("cargo:rerun-if-changed={}", icon_path.display());

    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon(icon_path.to_string_lossy().as_ref())
        .set("ProductName", "LANScanner")
        .set("FileDescription", "LANScanner Desktop Application")
        .set("OriginalFilename", "LANScanner.exe")
        .set("InternalName", "LANScanner");

    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        let repo_root = manifest_dir
            .parent()
            .and_then(|path| path.parent())
            .and_then(|path| path.parent())
            .expect("repo root should be discoverable from app crate");
        let binutils_dir = repo_root
            .join(".win-tools")
            .join("binutils")
            .join("usr")
            .join("bin");
        resource
            .set_windres_path(
                binutils_dir
                    .join("x86_64-w64-mingw32-windres")
                    .to_string_lossy()
                    .as_ref(),
            )
            .set_ar_path(
                binutils_dir
                    .join("x86_64-w64-mingw32-ar")
                    .to_string_lossy()
                    .as_ref(),
            );
    }

    if let Err(error) = resource.compile() {
        panic!("failed to compile Windows icon resource: {error}");
    }
}
