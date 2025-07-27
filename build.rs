extern crate embed_resource;

fn main() {
    #[cfg(target_family = "windows")]
    embed_resource::compile("tray.rc", embed_resource::NONE).manifest_optional().unwrap();
}
