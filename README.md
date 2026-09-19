# mersmaid

A small native desktop viewer for Mermaid diagrams. It uses the operating
system WebView through `wry` and embeds Mermaid, so rendering does not need a
network connection or a separate Chromium process.

## Install the CLI

```sh
cargo install mersmaid
```

## CLI usage

```sh
mersmaid 'flowchart LR; A-->B'
mersmaid diagram.mmd
cat diagram.mmd | mersmaid -
```

Right-drag to pan. Use a two-finger touchpad gesture to pan, and pinch or use
Ctrl+wheel to zoom.

## Library usage

```toml
[dependencies]
mersmaid = "0.1"
```

```rust,no_run
fn main() -> Result<(), Box<dyn std::error::Error>> {
    mersmaid::show("flowchart LR; A-->B")?;
    Ok(())
}
```

`show` blocks until the window closes and should be called from the process
main thread.

To build from source, run `cargo build --release`.

On Linux, building requires the WebKitGTK 4.1 and GTK 3 development packages.
