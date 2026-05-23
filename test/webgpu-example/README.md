# WebGPU demo

This is a simple demo to run librashader on WebGPU using `wasm32-unknown-unknown`

## Dependencies

librashader only supports a minimal subset when building on `wasm32-unknown-unknown`; you can not pull in the entire `librashader`
crate. 

Instead, compile only the minimal dependencies required to run librashader on the web.

```toml 
librashader-common = { path = "../librashader-common", version = "0.10.1", features = ["wgpu"] }
librashader-runtime-wgpu = { path = "../librashader-runtime-wgpu", version = "0.10.1", default-features = false, features = ["wgsl_preset_pack", "wgpu_webgpu"] }

# Required as shader presets must be prepacked on wasm32-unknown-unknown
librashader-pack = { path = "../librashader-pack", version = "0.10.1", default-features = false, features = ["serde"] }
```

## Generate the slangpkg

The `wasm32-unknown-unknown` target has no filesystem access nor is it able to use glslang, so shader presets must be prepackaged
and precompiled to WGSL `slangpkg`. Note that `slangpkg` has no stability guarantees at the moment so the format may change in the future.

You can create the `.wgsl.slangpkg` file with librashader-cli.

```bash
$ cargo run --release -p librashader-cli -- pack -p crt-royale.slangp -o crt-royale.wgsl.slangpkg -f msgpack -l wgsl
```

The demo uses crt-royale, but any shader should work. Put this `.wgsl.slangpkg` in the assets folder.

The `-l wgsl` parameter is required, or a GLSL `.slangpkg` will be created, which is not supported. This example uses msgpack,
but JSON `.slangpkg` packs can also be created.

## Building

You'll need `wasm-bindgen-cli`. Versions must match the `wasm-bindgen` crate
this demo depends on (currently the 0.2 series); install with:

```bash
$ cargo install wasm-bindgen-cli
```

Then build the wasm and emit JS glue:

```bash
$ cargo build --release --target wasm32-unknown-unknown
$ wasm-bindgen target/wasm32-unknown-unknown/release/librashader_web_demo.wasm --out-dir pkg --target web
```

Then serve index.html to see the demo.

