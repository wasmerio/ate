# WASM WebGL

The WASM Bus for using WebGL on tokera.sh

# Example

```rust
use wasm_bus_webgl::prelude::*;

fn main() -> Result<(), WebGlError> {
    let context = WebGl::new()?;

    context.clear_color(0.0, 0.0, 0.4, 1.0);
    context.clear(BufferBit::Color);

    std::thread::sleep(std::time::Duration::from_secs(4));

    Ok(())
}
```

# Testing

You can test your WASI program by uploading it to wapm.io and then heading over to the Tokera Shell

https://tokera.sh
