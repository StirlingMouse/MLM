use std::time::SystemTime;

extern crate embed_resource;

fn main() {
    let now = SystemTime::now();
    println!(
        "cargo:rustc-env=DATE={}",
        now.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    #[cfg(target_family = "windows")]
    embed_resource::compile("tray.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
