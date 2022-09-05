/*
 * thebestofcmu
 * Copyright © 2022 Anand Beh
 *
 * thebestofcmu is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * thebestofcmu is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with thebestofcmu. If not, see <https://www.gnu.org/licenses/>
 * and navigate to version 3 of the GNU Affero General Public License.
 */

use std::future::Future;
use std::net::SocketAddr;
use async_std::sync::Arc;
use async_std::net::TcpListener;
use eyre::Result;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper::body::HttpBody;
use hyper::http::{request, version};
use hyper::service::{make_service_fn, service_fn};
use rustls::ServerConfig;
use thebestofcmu_common::{ClientRSVP, PostPath};
use crate::database::Database;
use crate::method::AllowedMethod;
use crate::website::Website;

pub struct App {
    pub database: Database,
    pub website: Website
}

macro_rules! start_server_using {
    ($app:expr, $shutdown_future:expr, $listener:expr) => {
        Server::builder($listener)
            .executor(compat::HyperExecutor)
            .serve(make_service_fn(move |_| {
                let app = $app.clone();
                async {
                    Ok::<_, eyre::Report>(service_fn(move |request: Request<Body>| {
                        let app = app.clone();
                        async move { (&app).handle_request(request).await }
                    }))
                }
            }))
            .with_graceful_shutdown($shutdown_future)
            .await
    }
}

impl App {
    pub async fn start_server<F>(self,
                                 socket: SocketAddr,
                                 tls: Option<Arc<ServerConfig>>,
                                 shutdown_future: F) -> Result<()>
        where F: Future<Output=()> {

        let app = Arc::new(self);

        let listener = TcpListener::bind(&socket).await?;
        let listener = compat::HyperListener::new(&listener);
        log::info!("Bound to socket {}", socket);

        Ok(if let Some(tls) = tls {
            start_server_using!(app, shutdown_future, tls::TlsAcceptor::new(tls, listener))
        } else {
            start_server_using!(app, shutdown_future, listener)
        }?)
    }

    async fn handle_request(&self, request: Request<Body>) -> Result<Response<Body>> {
        let (parts, body) = request.into_parts();
        let method = AllowedMethod::find_from(&parts.method);
        match method {
            None => {
                AllowedMethod::method_not_alllowed(parts.version)
            },
            Some(AllowedMethod::GET) | Some(AllowedMethod::HEAD) => {
                self.yield_site(parts, body).await
            },
            Some(AllowedMethod::POST) => {
                Ok(match self.website.validate_post_path(parts.uri) {
                    None => {
                        Response::builder()
                            .version(parts.version)
                            .status(StatusCode::NOT_FOUND)
                            .body(Body::from("Non-existent POST path"))?
                    }
                    Some(PostPath::EnterRsvp) => {
                        match self.enter_rsvp(parts.version, body).await {
                            Err(e) => {
                                log::warn!("Miscellaneous error: {}", e);
                                Response::builder()
                                    .version(parts.version)
                                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                                    .body(Body::from("Miscellaneous error"))?
                            },
                            Ok(response) => response
                        }
                    }
                })
            }
        }
    }

    async fn yield_site(&self,
                        request_parts: request::Parts,
                        request_body: Body) -> Result<Response<Body>> {
        if !request_body.is_end_stream() {
            // Check if body is empty to conform to HTTP specification
            log::debug!("Received HTTP request with non-empty body: {:?}", &request_parts);
            return Ok(Response::builder()
                .version(request_parts.version)
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("A request must have an empty body"))?);
        }
        let body = if &request_parts.method == &Method::HEAD {
            // HEAD requests yield empty bodies
            Body::empty()
        } else {
            match self.website.yield_site_body(request_parts.uri.clone()).await {
                Some(body) => body,
                None => {
                    log::debug!("Not found: {}", request_parts.uri);
                    let msg = "According to my book-keeping, that page does not exist.";
                    return Ok(Response::builder()
                        .version(request_parts.version)
                        .status(StatusCode::NOT_FOUND)
                        .body(Body::from(msg))?);
                }
            }
        };
        Ok(Response::builder()
            .version(request_parts.version)
            .status(StatusCode::OK)
            .body(body)?)
    }

    async fn enter_rsvp(&self, version: version::Version, body: Body) -> Result<Response<Body>> {
        Ok(match ClientRSVP::decode(body).await {
            Err(e) => {
                log::warn!("Received bad client data: {}", e);
                Response::builder()
                    .version(version)
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Unable to parse RSVP json"))?
            }
            Ok(rsvp) => {
                match self.database.insert_rsvp(rsvp).await {
                    Err(e) => {
                        log::error!("Database error: {}", e);
                        Response::builder()
                            .version(version)
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::from("Database error"))?
                    },
                    Ok(response) => {
                        Response::builder()
                            .version(version)
                            .status(StatusCode::ACCEPTED)
                            .body(Body::from(serde_json::to_string(&response)?))?
                    }
                }
            }
        })
    }


}

mod compat {
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use async_std::io;
    use async_std::net::{self, TcpListener, TcpStream};
    use async_std::prelude::*;
    use async_std::task;
    use hyper::server::accept::Accept;

    #[derive(Clone)]
    pub struct HyperExecutor;

    impl<F> hyper::rt::Executor<F> for HyperExecutor
        where
            F: Future + Send + 'static,
            F::Output: Send + 'static,
    {
        fn execute(&self, fut: F) {
            task::spawn(fut);
        }
    }

    pub struct HyperListener<'listener> {
        incoming: net::Incoming<'listener>,
    }

    impl<'listener> HyperListener<'listener> {
        pub fn new(listener: &'listener TcpListener) -> Self {
            Self {
                incoming: listener.incoming(),
            }
        }
    }

    impl Accept for HyperListener<'_> {
        type Conn = HyperStream;
        type Error = io::Error;

        fn poll_accept(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
        ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
            let stream = task::ready!(Pin::new(&mut self.incoming).poll_next(cx)).unwrap()?;
            Poll::Ready(Some(Ok(HyperStream(stream))))
        }
    }

    pub struct HyperStream(TcpStream);

    impl tokio::io::AsyncRead for HyperStream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let bytes =
                task::ready!(Pin::new(&mut self.0).poll_read(cx, buf.initialize_unfilled())?);
            buf.advance(bytes);
            Poll::Ready(Ok(()))
        }
    }

    impl tokio::io::AsyncWrite for HyperStream {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            Pin::new(&mut self.0).poll_write(cx, buf)
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
            Pin::new(&mut self.0).poll_flush(cx)
        }

        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
            Pin::new(&mut self.0).poll_close(cx)
        }
    }
}

mod tls {
    use std::future::Future;
    use std::io;
    use std::pin::Pin;
    use async_std::sync::Arc;
    use std::task::{Context, Poll};
    use async_std::task::ready;
    use hyper::server::accept::Accept;
    use rustls::ServerConfig;
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    use crate::app::compat::{HyperListener, HyperStream};

    enum State {
        Handshaking(tokio_rustls::Accept<HyperStream>),
        Streaming(tokio_rustls::server::TlsStream<HyperStream>),
    }

    // tokio_rustls::server::TlsStream doesn't expose constructor methods,
    // so we have to TlsAcceptor::accept and handshake to have access to it
    // TlsStream implements AsyncRead/AsyncWrite handshaking tokio_rustls::Accept first
    pub struct TlsStream {
        state: State,
    }

    impl TlsStream {
        fn new(stream: HyperStream, config: Arc<ServerConfig>) -> TlsStream {
            let accept = tokio_rustls::TlsAcceptor::from(config).accept(stream);
            TlsStream {
                state: State::Handshaking(accept),
            }
        }
    }

    impl AsyncRead for TlsStream {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &mut ReadBuf,
        ) -> Poll<io::Result<()>> {
            let pin = self.get_mut();
            match pin.state {
                State::Handshaking(ref mut accept) => match ready!(Pin::new(accept).poll(cx)) {
                    Ok(mut stream) => {
                        let result = Pin::new(&mut stream).poll_read(cx, buf);
                        pin.state = State::Streaming(stream);
                        result
                    }
                    Err(err) => Poll::Ready(Err(err)),
                },
                State::Streaming(ref mut stream) => Pin::new(stream).poll_read(cx, buf),
            }
        }
    }

    impl AsyncWrite for TlsStream {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            let pin = self.get_mut();
            match pin.state {
                State::Handshaking(ref mut accept) => match ready!(Pin::new(accept).poll(cx)) {
                    Ok(mut stream) => {
                        let result = Pin::new(&mut stream).poll_write(cx, buf);
                        pin.state = State::Streaming(stream);
                        result
                    }
                    Err(err) => Poll::Ready(Err(err)),
                },
                State::Streaming(ref mut stream) => Pin::new(stream).poll_write(cx, buf),
            }
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            match self.state {
                State::Handshaking(_) => Poll::Ready(Ok(())),
                State::Streaming(ref mut stream) => Pin::new(stream).poll_flush(cx),
            }
        }

        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            match self.state {
                State::Handshaking(_) => Poll::Ready(Ok(())),
                State::Streaming(ref mut stream) => Pin::new(stream).poll_shutdown(cx),
            }
        }
    }

    pub struct TlsAcceptor<'l> {
        config: Arc<ServerConfig>,
        listener: HyperListener<'l>,
    }

    impl<'l> TlsAcceptor<'l> {
        pub fn new(config: Arc<ServerConfig>, listener: HyperListener<'l>) -> Self {
            Self { config, listener }
        }
    }

    impl<'l> Accept for TlsAcceptor<'l> {
        type Conn = TlsStream;
        type Error = io::Error;

        fn poll_accept(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
            let pin = self.get_mut();
            match ready!(Pin::new(&mut pin.listener).poll_accept(cx)) {
                Some(Ok(stream)) => Poll::Ready(Some(Ok(TlsStream::new(stream, pin.config.clone())))),
                Some(Err(e)) => Poll::Ready(Some(Err(e))),
                None => Poll::Ready(None),
            }
        }
    }
}