//! Build script: stage the embedded font, and embed the Windows icon and
//! version metadata.
//!
//! The UI embeds one CJK font so translated text renders everywhere. Large
//! assets do not belong in git, so the font is not committed: the build
//! finds a copy already installed on the machine, and otherwise fetches
//! the official release from GitHub into the build directory, where it is
//! cached like any other build artifact.
//!
//! Windows additionally reads the icon and version information from
//! resources compiled into the executable, so a packaged build shows the
//! right icon in Explorer and the taskbar without shipping files alongside
//! it. Other platforms take the icon from the window at runtime, so the
//! resource step does nothing there.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Where the staged font is written, relative to the build output directory.
/// `src/ui/fonts.rs` includes it from this exact location.
const STAGED_FONT: &str = "LXGWNeoXiHeiPlus.ttf";

/// The release the font is fetched from when no local copy exists.
/// Releases redirect to a CDN, so the fetcher must follow redirects.
const FONT_URL: &str =
    "https://github.com/lxgw/LxgwNeoXiHei/releases/download/v1.305/LXGWNeoXiHeiPlus.ttf";

fn main() {
    stage_font();
    #[cfg(target_os = "windows")]
    windows_resources();
}

/// Put the font at `OUT_DIR/LXGWNeoXiHeiPlus.ttf`.
///
/// Search order: the untracked `assets/` copy, which a developer can place
/// by hand, then the system font directories, then a fetch from the
/// release. A local copy avoids the network; the fetch keeps a clean CI
/// runner building without manual setup.
fn stage_font() {
    println!("cargo:rerun-if-changed=assets/{STAGED_FONT}");
    println!("cargo:rerun-if-changed=build.rs");

    let out = Path::new(&std::env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join(STAGED_FONT);
    if out.exists() && valid_ttf(&out) {
        // Already staged and sane: only a changed source should restage.
        return;
    }
    if let Some(local) = local_copy() {
        std::fs::copy(&local, &out)
            .unwrap_or_else(|error| panic!("could not stage {}: {error}", local.display()));
        assert!(valid_ttf(&out), "{} is not a readable TTF", local.display());
        eprintln!("yalt build: font staged from {}", local.display());
        return;
    }
    fetch(&out);
    assert!(valid_ttf(&out), "the fetched font is not a readable TTF");
    eprintln!("yalt build: font fetched from the {FONT_URL} release");
}

/// A usable copy already on this machine, newest search paths first.
fn local_copy() -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    dirs.push(PathBuf::from("assets"));
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let home = PathBuf::from(home);
        if cfg!(target_os = "windows") {
            if let Some(local) = std::env::var_os("LOCALAPPDATA") {
                dirs.push(Path::new(&local).join("Microsoft/Windows/Fonts"));
            }
            if let Some(windir) = std::env::var_os("WINDIR") {
                dirs.push(Path::new(&windir).join("Fonts"));
            }
        } else if cfg!(target_os = "macos") {
            dirs.push(home.join("Library/Fonts"));
            dirs.push(PathBuf::from("/Library/Fonts"));
            dirs.push(PathBuf::from("/System/Library/Fonts"));
        } else {
            dirs.push(home.join(".local/share/fonts"));
            dirs.push(home.join(".fonts"));
            dirs.push(PathBuf::from("/usr/share/fonts"));
        }
    }
    // Recursive: Linux distributes fonts into family subdirectories.
    dirs.iter()
        .flat_map(|dir| walk(dir, STAGED_FONT))
        .find(|path| valid_ttf(path))
}

/// Depth-limited search for one file name under a directory.
fn walk(dir: &Path, name: &str) -> Vec<PathBuf> {
    fn inner(dir: &Path, name: &str, depth: u8, hits: &mut Vec<PathBuf>) {
        if depth == 0 {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                inner(&path, name, depth - 1, hits);
            } else if path.file_name().is_some_and(|f| f == name) {
                hits.push(path);
            }
        }
    }
    let mut hits = Vec::new();
    inner(dir, name, 4, &mut hits);
    hits
}

/// Whether a file starts with a TrueType or OpenType scale magic.
///
/// A failed or redirected download leaves HTML or a stub here, so the
/// build must reject it instead of embedding a broken font.
fn valid_ttf(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    let Some(head) = bytes.get(..4) else {
        return false;
    };
    matches!(head, [0x00, 0x01, 0x00, 0x00] | b"OTTO" | b"true" | b"ttcf")
        && bytes.len() > 1_000_000
}

/// Download the font with curl, which ships with Windows 10+, macOS, and
/// the CI runners, so no new dependency is needed.
fn fetch(out: &Path) {
    let status = Command::new("curl")
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--output",
        ])
        .arg(out)
        .arg(FONT_URL)
        .status();
    match status {
        Ok(status) if status.success() => {}
        Ok(status) => panic!(
            "could not fetch the font (curl exit {}); place {STAGED_FONT} in assets/ to build offline",
            status.code().unwrap_or(-1)
        ),
        Err(error) => {
            panic!("curl is unavailable to fetch the font: {error}; place {STAGED_FONT} in assets/")
        }
    }
}

/// Compile the icon and version metadata into the executable.
///
/// The file and product versions come from the package version, which
/// `WindowsResource::new` reads from Cargo; only the strings that differ from
/// the package defaults are set here.
#[cfg(target_os = "windows")]
fn windows_resources() {
    println!("cargo:rerun-if-changed=build/windows/icon.ico");

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
