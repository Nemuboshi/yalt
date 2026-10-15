//! Build script: embed the Windows icon and version metadata.
//!
//! Windows reads the icon and version information from resources compiled into
//! the executable, so a packaged build shows the right icon in Explorer and
//! the taskbar without shipping files alongside it. Other platforms take the
//! icon from the window at runtime, so this does nothing there.

fn main() {
    #[cfg(target_os = "windows")]
    windows_resources();
}

/// Compile the icon and version metadata into the executable.
///
/// The file and product versions come from the package version, which
/// `WindowsResource::new` reads from Cargo; only the strings that differ from
/// the package defaults are set here.
#[cfg(target_os = "windows")]
fn windows_resources() {
    println!("cargo:rerun-if-changed=build/windows/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");

    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon("build/windows/icon.ico")
        // The package description is a sentence; Windows shows this string as
        // the application name in dialogs and the task manager.
        .set("FileDescription", "yalt translator")
        .set("CompanyName", "yalt")
        .set("LegalCopyright", "Copyright 2026 yalt")
        .set("OriginalFilename", "yalt.exe");
    if let Err(error) = resource.compile() {
        // A missing resource compiler must not break the build: the window
        // icon is also set at runtime, so only the shell metadata is lost.
        println!("cargo:warning=could not embed Windows resources: {error}");
    }
}
