use std::{
    borrow::Cow,
    env,
    error::Error,
    fs,
    io::{self, Read},
    path::Path,
};

use tao::{
    dpi::LogicalSize,
    event::Event,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

const MERMAID_JS: &str = include_str!("../assets/mermaid.min.js");
const PAGE_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <style>
    :root { color-scheme: light dark; font-family: system-ui, sans-serif; }
    * { box-sizing: border-box; }
    html, body { width: 100%; height: 100%; margin: 0; overflow: hidden; }
    body { background: #f7f7f8; color: #202124; }
    #viewport { width: 100%; height: 100%; overflow: auto; padding: 28px; }
    #diagram { display: grid; min-width: 100%; min-height: 100%; place-items: center; }
    #diagram svg { max-width: none !important; height: auto; }
    #error {
      display: none; margin: 0; padding: 24px; white-space: pre-wrap;
      color: #b42318; font: 14px/1.5 ui-monospace, monospace;
    }
    @media (prefers-color-scheme: dark) {
      body { background: #181a1b; color: #e8eaed; }
      #error { color: #ff8a80; }
    }
  </style>
</head>
<body>
  <main id="viewport"><div id="diagram"></div><pre id="error"></pre></main>
  <script src="/mermaid.min.js"></script>
  <script>
    document.addEventListener('DOMContentLoaded', async () => {
      const diagram = document.getElementById('diagram');
      const error = document.getElementById('error');
      try {
        mermaid.initialize({
          startOnLoad: false,
          securityLevel: 'strict',
          theme: matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'default'
        });
        const { svg, bindFunctions } = await mermaid.render('mersmaid-diagram', window.__MERSMAID_SOURCE__);
        diagram.innerHTML = svg;
        bindFunctions?.(diagram);
      } catch (reason) {
        diagram.style.display = 'none';
        error.style.display = 'block';
        error.textContent = reason instanceof Error ? reason.message : String(reason);
      }
    });
  </script>
</body>
</html>"#;

fn main() -> Result<(), Box<dyn Error>> {
    let Some(source) = read_source()? else {
        print_help();
        return Ok(());
    };

    configure_linux_cursor();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("mersmaid")
        .with_inner_size(LogicalSize::new(800, 560))
        .build(&event_loop)?;
    let init_script = format!(
        "window.__MERSMAID_SOURCE__ = {};",
        serde_json::to_string(&source)?
    );
    let builder = WebViewBuilder::new()
        .with_custom_protocol("mersmaid".into(), |_webview_id, request| {
            let (body, content_type, status) = match request.uri().path() {
                "/" | "/index.html" => (
                    Cow::Borrowed(PAGE_HTML.as_bytes()),
                    "text/html; charset=utf-8",
                    200,
                ),
                "/mermaid.min.js" => (
                    Cow::Borrowed(MERMAID_JS.as_bytes()),
                    "text/javascript; charset=utf-8",
                    200,
                ),
                _ => (
                    Cow::Borrowed(b"Not found".as_slice()),
                    "text/plain; charset=utf-8",
                    404,
                ),
            };

            wry::http::Response::builder()
                .status(status)
                .header("Content-Type", content_type)
                .header("Cache-Control", "public, max-age=31536000, immutable")
                .body(body)
                .expect("valid asset response")
        })
        .with_initialization_script(init_script)
        .with_url("mersmaid://localhost/index.html");

    #[cfg(target_os = "linux")]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let container = window
            .default_vbox()
            .ok_or_else(|| io::Error::other("window has no GTK container"))?;
        builder.build_gtk(container)?
    };
    #[cfg(not(target_os = "linux"))]
    let webview = builder.build(&window)?;

    event_loop.run(move |event, _, control_flow| {
        let _keep_alive = (&window, &webview);
        *control_flow = ControlFlow::Wait;
        if matches!(
            event,
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            }
        ) {
            *control_flow = ControlFlow::Exit;
        }
    });
}

#[cfg(target_os = "linux")]
fn configure_linux_cursor() {
    if env::var_os("XCURSOR_SIZE").is_none() {
        // This runs before GTK/WebKitGTK or any other threads are initialized.
        unsafe { env::set_var("XCURSOR_SIZE", "24") };
    }
}

#[cfg(not(target_os = "linux"))]
fn configure_linux_cursor() {}

fn read_source() -> Result<Option<String>, Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || matches!(args.first().map(String::as_str), Some("-h" | "--help")) {
        return Ok(None);
    }

    if args.len() == 1 && args[0] == "-" {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source)?;
        return Ok(Some(source));
    }

    if args.len() == 1 && Path::new(&args[0]).is_file() {
        return Ok(Some(fs::read_to_string(&args[0])?));
    }

    Ok(Some(args.join(" ")))
}

fn print_help() {
    eprintln!(
        "Usage:\n  mersmaid '<mermaid code>'\n  mersmaid <diagram.mmd>\n  cat diagram.mmd | mersmaid -"
    );
}
