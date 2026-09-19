# mersmaid

A small native desktop viewer for Mermaid diagrams. It uses the operating
system WebView through `wry` and embeds Mermaid, so rendering does not need a
network connection or a separate Chromium process.

```sh
cargo run -- 'flowchart LR; A-->B'
cargo run -- diagram.mmd
cat diagram.mmd | cargo run -- -
```

For normal use, build the optimized binary with `cargo build --release`.

On Linux, building requires the WebKitGTK 4.1 and GTK 3 development packages.
