extern crate embed_resource;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(target_family = "windows")]
    {
        println!("cargo:rerun-if-changed=tray.rc");
        embed_resource::compile("tray.rc", embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
