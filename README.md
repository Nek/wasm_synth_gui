### What it is?
Cross desktop and browsers audio DSP app templae with GUI in Rust. It's just a setup of a project. I'm not the author of any libraries used.

### Features
- an audio DSP library that works both in the browsers (Firefox, Chrome, Safari, probably Edge, not IE) and on the desktop (only macOS for now);
- cross-platform UI, OpenGL on a desktop, WebGL in a browser;
- multithreaded on all the platforms;
- same code for all the platforms, there is a bit of glue code, you don't have to touch it.

### Requirements
- nightly Rust;
- trunk;
- some experimental features, check `.cargo/config.toml`.

### Testing locally

#### Desktop

`cargo run --release`

#### Web

`trunk build --release` and use a local web server or web hosting.

This is due to the browser sandbox limitations for running wasm and web audio.
Configs for Caddy and Netlify are included.

### Web Demo

[wasm-synth-gui.nikdudnik.com](https://wasm-synth-gui.nikdudnik.com/)

It will play a simple sinusoid signal.