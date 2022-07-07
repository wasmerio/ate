use wasm_bus_tty::prelude::*;

#[cfg(target_family = "wasm")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    wasm_bus::task::block_on(main_async())
}

#[cfg(not(target_family = "wasm"))]
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    main_async().await?;
    std::process::exit(0);
}

async fn main_async() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdin = Tty::stdin().await?;
    let mut stdout = Tty::stdout().await?;
    loop {
        if let Some(data) = stdin.read().await {
            if data.len() == 1 && data[0] == 120u8 {
                break;
            }
            stdout.write(data).await?;
            stdout.flush().await?;
        } else {
            break;
        }
    }
    Ok(())
}