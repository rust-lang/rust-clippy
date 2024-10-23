#![warn(clippy::unnecessary_shell_command)]

use std::process::Command;

fn main() {
    let _ = Command::new("ls");
    //~^ error: unnecessarily shelling out for trivial operation
    let _ = Command::new("curl");
    //~^ error: unnecessarily shelling out for trivial operation
    let _ = Command::new("wget");
    //~^ error: unnecessarily shelling out for trivial operation
    let _ = Command::new("sed");
    //~^ error: unnecessarily shelling out for trivial operation
    let _ = Command::new("jq");
    //~^ error: unnecessarily shelling out for trivial operation

    let _ = Command::new("ffmpeg");
}
