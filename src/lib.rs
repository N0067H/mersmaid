//! Native desktop viewer for Mermaid diagram source.
//!
//! [`show`] opens a window and blocks the calling thread until it closes. Call
//! it from the process main thread because desktop event loops generally
//! require main-thread access.

use std::{borrow::Cow, error::Error, io};

use tao::{
    dpi::LogicalSize,
    event::Event,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

const MERMAID_JS: &str = include_str!("../assets/mermaid.min.js");
const NOTO_SANS_KR: &[u8] = include_bytes!("../assets/NotoSansKR-wght.ttf");
const PAGE_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <style>
    @font-face {
      font-family: "Mersmaid Noto Sans KR";
      src: url("/NotoSansKR-wght.ttf") format("truetype");
      font-style: normal;
      font-weight: 100 900;
      font-display: block;
    }
    :root {
      color-scheme: light dark;
      font-family: "Mersmaid Noto Sans KR", system-ui, "Noto Sans CJK KR", "Noto Sans KR",
        "Apple SD Gothic Neo", "Malgun Gothic", Arial, sans-serif;
    }
    * { box-sizing: border-box; }
    html, body { width: 100%; height: 100%; margin: 0; overflow: hidden; }
    html, body, body * {
      cursor: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='18' height='24' viewBox='0 0 18 24'%3E%3Cpath d='M2 1.5v17l4.2-4.1 3.2 7.4 3.3-1.4-3.2-7.2h6.1z' fill='white' stroke='black' stroke-width='1.5' stroke-linejoin='round'/%3E%3C/svg%3E") 2 2, default !important;
    }
    body { background: #f7f7f8; color: #202124; }
    #titlebar {
      display: grid; grid-template-columns: 1fr auto; align-items: center;
      height: 34px; background: #ededee; user-select: none;
      border-bottom: 1px solid #d8d8da;
    }
    #drag-region { height: 100%; padding: 7px 12px; font-size: 13px; app-region: drag; }
    #window-actions { display: flex; height: 100%; }
    .window-button {
      width: 44px; height: 100%; border: 0; background: transparent;
      color: inherit; font: 18px/1 system-ui, sans-serif;
    }
    .window-button:hover { background: #d8d8da; }
    #close:hover { color: white; background: #c42b1c; }
    #viewport {
      position: relative; width: 100%; height: calc(100% - 34px); overflow: hidden;
    }
    #diagram {
      position: absolute; left: 0; top: 0;
    }
    #diagram svg { display: block; max-width: none !important; }
    #error {
      display: none; margin: 0; padding: 24px; white-space: pre-wrap;
      color: #b42318; font: 14px/1.5 ui-monospace, monospace;
    }
    @media (prefers-color-scheme: dark) {
      body { background: #181a1b; color: #e8eaed; }
      #titlebar { background: #242627; border-bottom-color: #343637; }
      .window-button:hover { background: #383a3b; }
      #error { color: #ff8a80; }
    }
  </style>
</head>
<body>
  <header id="titlebar">
    <div id="drag-region" data-drag-region>mersmaid</div>
    <div id="window-actions">
      <button class="window-button" id="minimize" aria-label="Minimize">&#8722;</button>
      <button class="window-button" id="maximize" aria-label="Maximize or restore">&#9633;</button>
      <button class="window-button" id="close" aria-label="Close">&#215;</button>
    </div>
  </header>
  <main id="viewport"><div id="diagram"></div><pre id="error"></pre></main>
  <script src="/mermaid.min.js"></script>
  <script>
    document.addEventListener('DOMContentLoaded', async () => {
      const dragRegion = document.getElementById('drag-region');
      dragRegion.addEventListener('mousedown', event => {
        if (event.button === 0) window.ipc.postMessage(event.detail === 2 ? 'maximize' : 'drag');
      });
      document.getElementById('minimize').addEventListener('click', () => window.ipc.postMessage('minimize'));
      document.getElementById('maximize').addEventListener('click', () => window.ipc.postMessage('maximize'));
      document.getElementById('close').addEventListener('click', () => window.ipc.postMessage('close'));

      const viewport = document.getElementById('viewport');
      const diagram = document.getElementById('diagram');
      let offsetX = 0;
      let offsetY = 0;
      let scale = 1;
      let svgElement = null;
      let svgWidth = 0;
      let svgHeight = 0;
      let pan = null;
      const updateLayout = () => {
        diagram.style.left = `${offsetX}px`;
        diagram.style.top = `${offsetY}px`;
        if (svgElement) {
          // Resize the SVG viewport itself instead of scaling a cached compositor layer.
          // This makes WebKit rasterize text and strokes again at the new resolution.
          svgElement.style.width = `${svgWidth * scale}px`;
          svgElement.style.height = `${svgHeight * scale}px`;
        }
      };
      viewport.addEventListener('pointerdown', event => {
        if (event.button !== 2) return;
        event.preventDefault();
        pan = {
          x: event.clientX,
          y: event.clientY,
          offsetX,
          offsetY
        };
        viewport.setPointerCapture(event.pointerId);
      });
      viewport.addEventListener('pointermove', event => {
        if (!pan) return;
        offsetX = pan.offsetX + event.clientX - pan.x;
        offsetY = pan.offsetY + event.clientY - pan.y;
        updateLayout();
      });
      const stopPanning = event => {
        if (!pan) return;
        pan = null;
        if (viewport.hasPointerCapture(event.pointerId)) viewport.releasePointerCapture(event.pointerId);
      };
      viewport.addEventListener('pointerup', stopPanning);
      viewport.addEventListener('pointercancel', stopPanning);
      viewport.addEventListener('contextmenu', event => event.preventDefault());
      viewport.addEventListener('wheel', event => {
        event.preventDefault();
        if (event.ctrlKey) {
          const rect = viewport.getBoundingClientRect();
          const x = event.clientX - rect.left;
          const y = event.clientY - rect.top;
          const nextScale = Math.min(8, Math.max(0.02, scale * Math.exp(-event.deltaY * 0.002)));
          const ratio = nextScale / scale;
          offsetX = x - (x - offsetX) * ratio;
          offsetY = y - (y - offsetY) * ratio;
          scale = nextScale;
        } else {
          offsetX -= event.deltaX;
          offsetY -= event.deltaY;
        }
        updateLayout();
      }, { passive: false });

      const error = document.getElementById('error');
      try {
        const fontFamily = '"Mersmaid Noto Sans KR", system-ui, "Noto Sans CJK KR", "Noto Sans KR", '
          + '"Apple SD Gothic Neo", "Malgun Gothic", Arial, sans-serif';
        mermaid.initialize({
          startOnLoad: false,
          securityLevel: 'strict',
          theme: matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'default',
          fontFamily,
          themeVariables: { fontFamily }
        });
        const { svg, bindFunctions } = await mermaid.render('mersmaid-diagram', window.__MERSMAID_SOURCE__);
        diagram.innerHTML = svg;
        bindFunctions?.(diagram);
        svgElement = diagram.querySelector('svg');
        if (svgElement) {
          const viewBox = svgElement.viewBox.baseVal;
          svgWidth = viewBox.width || svgElement.getBoundingClientRect().width;
          svgHeight = viewBox.height || svgElement.getBoundingClientRect().height;

          const margin = 28;
          const availableWidth = Math.max(1, viewport.clientWidth - margin * 2);
          const availableHeight = Math.max(1, viewport.clientHeight - margin * 2);
          scale = Math.min(1, availableWidth / svgWidth, availableHeight / svgHeight);
          offsetX = (viewport.clientWidth - svgWidth * scale) / 2;
          offsetY = (viewport.clientHeight - svgHeight * scale) / 2;
          updateLayout();
        }
      } catch (reason) {
        diagram.style.display = 'none';
        error.style.display = 'block';
        error.textContent = reason instanceof Error ? reason.message : String(reason);
      }
    });
  </script>
</body>
</html>"#;

enum UserEvent {
    Drag,
    Minimize,
    Maximize,
    Close,
}

/// Opens a native window and renders `source` as a Mermaid diagram.
///
/// This function runs the desktop event loop and returns after the window is
/// closed. It should be called from the process main thread.
pub fn show(source: impl Into<String>) -> Result<(), Box<dyn Error>> {
    let source = source.into();
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let window = WindowBuilder::new()
        .with_title("mersmaid")
        .with_decorations(false)
        .with_inner_size(LogicalSize::new(800, 560))
        .build(&event_loop)?;
    let init_script = format!(
        "window.__MERSMAID_SOURCE__ = {};",
        serde_json::to_string(&source)?
    );
    let proxy = event_loop.create_proxy();
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
                "/NotoSansKR-wght.ttf" => (Cow::Borrowed(NOTO_SANS_KR), "font/ttf", 200),
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
        .with_ipc_handler(move |request| {
            let event = match request.body().as_str() {
                "drag" => Some(UserEvent::Drag),
                "minimize" => Some(UserEvent::Minimize),
                "maximize" => Some(UserEvent::Maximize),
                "close" => Some(UserEvent::Close),
                _ => None,
            };
            if let Some(event) = event {
                let _ = proxy.send_event(event);
            }
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
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            }
            | Event::UserEvent(UserEvent::Close) => *control_flow = ControlFlow::Exit,
            Event::UserEvent(UserEvent::Drag) => {
                let _ = window.drag_window();
            }
            Event::UserEvent(UserEvent::Minimize) => window.set_minimized(true),
            Event::UserEvent(UserEvent::Maximize) => {
                window.set_maximized(!window.is_maximized());
            }
            _ => {}
        }
    });
}
