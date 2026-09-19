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

## Fonts and non-Latin text

Mersmaid bundles Noto Sans KR and uses it before a cross-platform system font
fallback stack. Korean labels therefore require no separate font installation
or user configuration, including on minimal Linux installations. Other scripts
use an installed system font when the bundled font does not contain their
glyphs.

The bundled Noto Sans KR font is licensed under the SIL Open Font License 1.1;
see `assets/NOTO_SANS_KR_LICENSE`.

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

## Publishing

Releases are published by the `Release crate` GitHub Actions workflow. Before
running it for the first time, add a crates.io API token as the repository
Actions secret `CARGO_REGISTRY_TOKEN`. The token must be allowed to publish the
`mersmaid` crate. The repository must also allow GitHub Actions to write
contents; if the default branch is protected, allow the workflow to push its
release commit.

Run the workflow manually, enter the next version without a `v` prefix (for
example, `0.1.1`), and confirm the release. The workflow updates `Cargo.toml`
and `Cargo.lock`, runs the tests, checks that the bundled font and its license
are packaged, pushes the version commit, publishes to crates.io, and pushes the
matching Git tag.
