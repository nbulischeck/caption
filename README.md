# GIF Caption Creator

A web application that allows users to add customizable captions to GIF files using WebAssembly and Rust.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- A modern web browser

## Building the Project

1. Install the required tools:
   ```bash
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # Install wasm-pack
   cargo install wasm-pack
   ```

2. Build the WebAssembly module:
   ```bash
   wasm-pack build --target web
   ```

3. Serve the application:
   You can use any static file server. For example, with Python:
   ```bash
   # Python 3
   python -m http.server 8080
   ```

4. Open your browser and navigate to `http://localhost:8080`

## Usage

1. Drag and drop a GIF file onto the drop zone or click to select a file
2. Enter your caption text in the text input
3. Customize the caption:
   - Choose a font family
   - Adjust the font size
   - Position the caption using the X and Y sliders
4. Click "Process GIF" to add the caption
5. Click "Download Result" to save the captioned GIF