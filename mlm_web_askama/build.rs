use std::time::SystemTime;

fn main() {
    let now = SystemTime::now();
    println!(
        "cargo:rustc-env=DATE={}",
        now.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
}
