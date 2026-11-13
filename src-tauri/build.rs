use std::path::Path;

fn main() {
    tauri_build::build();

    // tauri-build embeds the application manifest (comctl32 v6, DPI awareness) into
    // `bins` only. Test harnesses link the same Tauri code, so they need the manifest
    // as well or Windows resolves comctl32 v5 and startup fails on TaskDialogIndirect.
    let Ok(out_dir) = std::env::var("OUT_DIR") else {
        return;
    };
    let resource = Path::new(&out_dir).join("libresource.a");
    if resource.exists() {
        println!("cargo:rustc-link-arg-tests={}", resource.display());
    }
}
