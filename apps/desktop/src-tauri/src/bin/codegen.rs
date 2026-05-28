//! Standalone codegen entry point. Writes `packages/ipc/src/generated.ts`
//! from the `tauri-specta` builder shared with `lib.rs`. Invoked from the
//! repo via `pnpm --filter @cellar/ipc codegen`.

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let builder = cellar_desktop_lib::commands::builder();
    let exporter = cellar_desktop_lib::commands::typescript_exporter();

    // Resolve the output relative to this crate so `cargo run` from anywhere
    // still lands the file in `packages/ipc/src/generated.ts`.
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target = crate_dir
        .join("..")
        .join("..")
        .join("..")
        .join("packages")
        .join("ipc")
        .join("src")
        .join("generated.ts");

    let target = target.canonicalize().unwrap_or(target);

    builder.export(exporter, &target)?;
    println!("wrote {}", target.display());
    Ok(())
}
