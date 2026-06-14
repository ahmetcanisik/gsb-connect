//! Build script.
//!
//! On Windows it embeds version information and, when present, an application
//! icon into the executable using the maintained `winresource` crate. This only
//! changes how the `.exe` looks in Explorer's Properties > Details tab and gives
//! it an icon; it does NOT replace code signing and does not affect runtime
//! behavior.
//!
//! The `#[cfg(windows)]` gate matches the build-dependency gate in `Cargo.toml`
//! (build dependencies are evaluated for the host), so on non-Windows hosts this
//! is a clean no-op and the project still builds for development.

fn main() {
    #[cfg(windows)]
    embed_windows_resources();
}

#[cfg(windows)]
fn embed_windows_resources() {
    use std::path::Path;

    // Where to drop a real icon. See assets/README.md for sizing guidance. The
    // build still succeeds (version info only) when no icon file is present.
    const ICON_PATH: &str = "assets/app.ico";

    let mut res = winresource::WindowsResource::new();

    // Strings shown in the file's Properties > Details tab. Edit CompanyName to
    // your own publisher name. The version fields default from Cargo's package
    // version (CARGO_PKG_VERSION), so bumping `version` in Cargo.toml is enough.
    res.set("ProductName", "GSB Connect");
    res.set("FileDescription", "GSB Connect");
    res.set("CompanyName", "GSB Connect Project");
    res.set("LegalCopyright", "Distributed under the MIT License.");

    if Path::new(ICON_PATH).exists() {
        res.set_icon(ICON_PATH);
        println!("cargo:rerun-if-changed={ICON_PATH}");
    } else {
        println!(
            "cargo:warning=No application icon found at {ICON_PATH}; embedding version \
             info only. Drop a real app.ico there to add an icon."
        );
    }

    // Resource embedding is cosmetic, so a failure here (e.g. no resource
    // compiler on PATH) must not break the build of the application itself.
    if let Err(error) = res.compile() {
        println!("cargo:warning=Failed to embed Windows resources: {error}");
    }
}
