//! Linux desktop integration: install .desktop file and icons on first run
//! so the app appears in application launchers after `cargo install`.

use std::{fs, path::PathBuf};

/// The .desktop file content, kept in sync with `.pkg/spotifoss.desktop`.
const DESKTOP_ENTRY: &str = "\
[Desktop Entry]
Type=Application
Name=Spotifoss
Comment=Fast and native Spotify client
GenericName=Music Player
Icon=spotifoss
TryExec=spotifoss
Exec=spotifoss %U
Terminal=false
MimeType=x-scheme-handler/spotifoss;
Categories=Audio;Music;Player;AudioVideo;
StartupWMClass=spotifoss
";

/// Icon assets embedded at compile time from spotifoss-gui/assets/.
const ICON_32: &[u8] = include_bytes!("../../assets/logo_32.png");
const ICON_64: &[u8] = include_bytes!("../../assets/logo_64.png");
const ICON_128: &[u8] = include_bytes!("../../assets/logo_128.png");
const ICON_256: &[u8] = include_bytes!("../../assets/logo_256.png");
const ICON_512: &[u8] = include_bytes!("../../assets/logo_512.png");
const ICON_SVG: &[u8] = include_bytes!("../../assets/logo.svg");

/// Install the .desktop file and icons into the user's local directories.
/// Safe to call on every launch -- only writes files that are missing or outdated.
#[cfg(target_os = "linux")]
pub fn ensure_desktop_integration() {
    let data_home = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            PathBuf::from(home).join(".local/share")
        });

    // Install .desktop file
    let apps_dir = data_home.join("applications");
    install_file(&apps_dir, "spotifoss.desktop", DESKTOP_ENTRY.as_bytes());

    // Install icons into hicolor theme
    let icons_base = data_home.join("icons/hicolor");
    let icon_sizes: &[(&str, &[u8])] = &[
        ("32x32", ICON_32),
        ("64x64", ICON_64),
        ("128x128", ICON_128),
        ("256x256", ICON_256),
        ("512x512", ICON_512),
    ];
    for (size, data) in icon_sizes {
        let dir = icons_base.join(size).join("apps");
        install_file(&dir, "spotifoss.png", data);
    }

    // Scalable SVG
    let svg_dir = icons_base.join("scalable/apps");
    install_file(&svg_dir, "spotifoss.svg", ICON_SVG);

    // Try to update the icon cache (best effort, not required)
    let _ = std::process::Command::new("gtk-update-icon-cache")
        .arg("-f")
        .arg("-t")
        .arg(icons_base)
        .output();

    // Try to update the desktop database (best effort)
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&apps_dir)
        .output();

    log::info!("desktop integration: installed .desktop file and icons");
}

#[cfg(not(target_os = "linux"))]
pub fn ensure_desktop_integration() {
    // Desktop integration is only needed on Linux.
    // macOS uses cargo-bundle / .app bundles.
    // Windows uses the .exe icon embedded by build.rs.
}

#[cfg(target_os = "linux")]
fn install_file(dir: &PathBuf, name: &str, content: &[u8]) {
    if let Err(err) = fs::create_dir_all(dir) {
        log::warn!("desktop integration: failed to create {dir:?}: {err}");
        return;
    }
    let path = dir.join(name);
    // Only write if missing or content differs (avoids unnecessary writes)
    if path.exists()
        && let Ok(existing) = fs::read(&path)
        && existing == content
    {
        return;
    }
    if let Err(err) = fs::write(&path, content) {
        log::warn!("desktop integration: failed to write {path:?}: {err}");
    }
}
