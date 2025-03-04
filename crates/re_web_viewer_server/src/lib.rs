//! Serves the web viewer wasm/html.
//!
//! ## Feature flags
#![doc = document_features::document_features!()]
//!

#![forbid(unsafe_code)]
#![warn(clippy::all, rust_2018_idioms)]
#![allow(clippy::manual_range_contains)]

use std::task::{Context, Poll};

use futures_util::future;
use hyper::{server::conn::AddrIncoming, service::Service, Body, Request, Response};

struct Svc {
    // NOTE: Optional because it is possible to have the `analytics` feature flag enabled
    // while at the same time opting-out of analytics at run-time.
    #[cfg(feature = "analytics")]
    analytics: Option<re_analytics::Analytics>,
}

impl Svc {
    #[cfg(feature = "analytics")]
    fn new() -> Self {
        let analytics = match re_analytics::Analytics::new(std::time::Duration::from_secs(2)) {
            Ok(analytics) => Some(analytics),
            Err(err) => {
                re_log::error!(%err, "failed to initialize analytics SDK");
                None
            }
        };
        Self { analytics }
    }

    #[cfg(not(feature = "analytics"))]
    fn new() -> Self {
        Self {}
    }

    #[cfg(feature = "analytics")]
    fn on_serve_wasm(&self) {
        if let Some(analytics) = &self.analytics {
            analytics.record(re_analytics::Event::append("serve_wasm"));
        }
    }
}

impl Service<Request<Body>> for Svc {
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    #[cfg(feature = "__ci")]
    fn call(&mut self, _req: Request<Body>) -> Self::Future {
        if false {
            self.on_serve_wasm(); // to silence warning about the function being unused
        }

        panic!("web_server compiled with '__ci' feature (or `--all-features`). DON'T DO THAT! It's only for the CI!");
    }

    #[cfg(not(feature = "__ci"))]
    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let rsp = Response::builder();

        let bytes = match req.uri().path() {
            "/" | "/index.html" => &include_bytes!("../web_viewer/index.html")[..],
            "/favicon.ico" => &include_bytes!("../web_viewer/favicon.ico")[..],
            "/sw.js" => &include_bytes!("../web_viewer/sw.js")[..],

            #[cfg(debug_assertions)]
            "/re_viewer.js" => &include_bytes!("../web_viewer/re_viewer_debug.js")[..],
            #[cfg(not(debug_assertions))]
            "/re_viewer.js" => &include_bytes!("../web_viewer/re_viewer.js")[..],

            "/re_viewer_bg.wasm" => {
                #[cfg(feature = "analytics")]
                self.on_serve_wasm();

                #[cfg(debug_assertions)]
                {
                    re_log::info_once!("Serving DEBUG web-viewer");
                    &include_bytes!("../web_viewer/re_viewer_debug_bg.wasm")[..]
                }
                #[cfg(not(debug_assertions))]
                {
                    &include_bytes!("../web_viewer/re_viewer_bg.wasm")[..]
                }
            }
            _ => {
                re_log::warn!("404 path: {}", req.uri().path());
                let body = Body::from(Vec::new());
                let rsp = rsp.status(404).body(body).unwrap();
                return future::ok(rsp);
            }
        };

        let body = Body::from(Vec::from(bytes));
        let rsp = rsp.status(200).body(body).unwrap();
        future::ok(rsp)
    }
}

struct MakeSvc;

impl<T> Service<T> for MakeSvc {
    type Response = Svc;
    type Error = std::io::Error;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, _: T) -> Self::Future {
        future::ok(Svc::new())
    }
}

// ----------------------------------------------------------------------------

/// Hosts the Web Viewer Wasm+HTML
pub struct WebViewerServer {
    server: hyper::Server<AddrIncoming, MakeSvc>,
}

impl WebViewerServer {
    pub fn new(port: u16) -> Self {
        let bind_addr = format!("0.0.0.0:{port}").parse().unwrap();
        let server = hyper::Server::bind(&bind_addr).serve(MakeSvc);
        Self { server }
    }

    pub async fn serve(self) -> anyhow::Result<()> {
        self.server.await?;
        Ok(())
    }
}
