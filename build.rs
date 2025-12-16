use std::process::Command;

fn main() {
    let status = Command::new("make")
        .current_dir("tools/bdfconv")
        .status()
        .expect("Failed to run make in ./tools/bdfconv");

    if !status.success() {
        panic!("make command failed in ./tools/bdfconv");
    }
}
