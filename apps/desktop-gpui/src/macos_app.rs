use std::path::{Path, PathBuf};

const INFO_PLIST: &[u8] = include_bytes!("../macos/Info.plist");
#[cfg(target_os = "macos")]
const ICON_ICNS: &[u8] = include_bytes!("../../desktop/src-tauri/icons/icon.icns");

enum LaunchLocation {
    AppBundle,
    Unbundled,
}

#[cfg(target_os = "macos")]
pub fn relaunch_from_app_bundle() {
    use std::os::unix::process::CommandExt;

    let exe = std::env::current_exe().expect("current executable");
    if matches!(launch_location(&exe), LaunchLocation::AppBundle) {
        return;
    }
    let bundled = stage_app_bundle(&exe, INFO_PLIST, ICON_ICNS).expect("stage Cellar.app");
    let err = std::process::Command::new(&bundled)
        .args(std::env::args().skip(1))
        .exec();
    panic!("relaunch Cellar.app: {err}");
}

#[cfg(not(target_os = "macos"))]
pub fn relaunch_from_app_bundle() {}

fn launch_location(exe: &Path) -> LaunchLocation {
    let macos = exe.parent();
    let contents = macos.and_then(Path::parent);
    let app = contents.and_then(Path::parent);
    match (macos, contents, app) {
        (Some(macos), Some(contents), Some(app))
            if macos.file_name().is_some_and(|name| name == "MacOS")
                && contents.file_name().is_some_and(|name| name == "Contents")
                && app.extension().is_some_and(|ext| ext == "app") =>
        {
            LaunchLocation::AppBundle
        }
        _ => LaunchLocation::Unbundled,
    }
}

fn stage_app_bundle(exe: &Path, plist: &[u8], icns: &[u8]) -> std::io::Result<PathBuf> {
    let app_dir = exe
        .parent()
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "executable has no parent")
        })?
        .join("Cellar.app");
    let contents = app_dir.join("Contents");
    let macos = contents.join("MacOS");
    let resources = contents.join("Resources");
    std::fs::create_dir_all(&macos)?;
    std::fs::create_dir_all(&resources)?;
    std::fs::write(contents.join("Info.plist"), plist)?;
    std::fs::write(resources.join("icon.icns"), icns)?;
    let dest = macos.join("Cellar");
    if needs_copy(exe, &dest) {
        std::fs::copy(exe, &dest)?;
    }
    Ok(dest)
}

fn needs_copy(src: &Path, dest: &Path) -> bool {
    let Ok(dest_meta) = dest.metadata() else {
        return true;
    };
    let Ok(src_meta) = src.metadata() else {
        return true;
    };
    dest_meta.len() != src_meta.len() || src_meta.modified().ok() > dest_meta.modified().ok()
}

#[cfg(test)]
mod tests {
    use super::{launch_location, stage_app_bundle, LaunchLocation, INFO_PLIST};
    use std::fs;
    use std::path::Path;

    #[test]
    fn unbundled_exe_is_unbundled() {
        assert!(matches!(
            launch_location(Path::new("/tmp/debug/cellar-desktop-gpui")),
            LaunchLocation::Unbundled
        ));
    }

    #[test]
    fn bundled_exe_is_app_bundle() {
        assert!(matches!(
            launch_location(Path::new("/tmp/debug/Cellar.app/Contents/MacOS/Cellar")),
            LaunchLocation::AppBundle
        ));
    }

    #[test]
    fn stages_plist_icon_and_binary() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("cellar-desktop-gpui");
        fs::write(&exe, b"fake-binary").unwrap();
        let dest = stage_app_bundle(&exe, INFO_PLIST, b"icns").unwrap();
        assert_eq!(dest, dir.path().join("Cellar.app/Contents/MacOS/Cellar"));
        assert_eq!(fs::read(&dest).unwrap(), b"fake-binary");
        let plist = fs::read_to_string(dir.path().join("Cellar.app/Contents/Info.plist")).unwrap();
        assert!(plist.contains("<key>CFBundleName</key>"));
        assert!(plist.contains("<key>CFBundleDisplayName</key>"));
        assert!(plist.contains("<string>Cellar</string>"));
        assert_eq!(
            fs::read(dir.path().join("Cellar.app/Contents/Resources/icon.icns")).unwrap(),
            b"icns"
        );
    }

    #[test]
    fn skips_copy_when_dest_is_current() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("cellar-desktop-gpui");
        fs::write(&exe, b"v1").unwrap();
        let dest = stage_app_bundle(&exe, b"plist", b"icns").unwrap();
        let first = fs::metadata(&dest).unwrap().modified().unwrap();
        let dest_again = stage_app_bundle(&exe, b"plist", b"icns").unwrap();
        assert_eq!(dest, dest_again);
        assert_eq!(fs::read(&dest).unwrap(), b"v1");
        assert_eq!(fs::metadata(&dest).unwrap().modified().unwrap(), first);
    }
}
