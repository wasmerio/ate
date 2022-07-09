use wasm_bus_process::prelude::*;

fn main() {
    let mut task = Command::new("ls")
            .stdin(Stdio::inherit())
            .stderr(Stdio::inherit())
            .stdout(Stdio::inherit())
            .spawn()
            .expect("ls command failed to start");
    task.wait().unwrap();
}
