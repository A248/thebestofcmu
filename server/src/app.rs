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

use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use async_std::net::TcpListener;
use eyre::Result;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use hyper::body::HttpBody;
use hyper::http::{request, version};
use hyper::service::{make_service_fn, service_fn};
use thebestofcmu_common::{ClientRSVP, PostPath};
use crate::database::Database;
use crate::method::AllowedMethod;
use crate::website::Website;

pub struct App {
    pub database: Database,
    pub website: Website
}

impl App {
    pub async fn start_server<F>(self,
                                        socket: SocketAddr,
                                        shutdown_future: F) -> Result<()>
        where F: Future<Output=()> {

        let app = Arc::new(self);
        let service_function = make_service_fn(move |_| {
            let app = app.clone();
            async {
                Ok::<_, Infallible>(service_fn(move |request: Request<Body>| {
                    let app = app.clone();
                    async move { (&app).handle_request(request).await }
                }))
            }
        });
        //let listener = TcpListener::bind(&socket).await?;
        log::info!("Bound to socket");
        /*let server = Server::builder(compat::HyperListener(listener))
            .executor(compat::HyperExecutor)
            .serve(service_function);*/
        tokio::runtime::
        tokio::spawn(async move {
            Server::try_bind(&socket)?
                .serve(service_function)
                .await?;
            Ok::<(), eyre::Report>(())
        });
        Ok::<(), eyre::Report>(())
        //Ok(server.with_graceful_shutdown(shutdown_future).await?)
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
    use async_compat::CompatExt;

    use async_std::io;
    use async_std::net::{TcpListener, TcpStream};
    use async_std::prelude::*;
    use async_std::task;

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

    pub struct HyperListener(pub TcpListener);

    impl hyper::server::accept::Accept for HyperListener {
        type Conn = async_compat::Compat<TcpStream>;
        type Error = io::Error;

        fn poll_accept(
            self: Pin<&mut Self>,
            cx: &mut Context,
        ) -> Poll<Option<Result<Self::Conn, Self::Error>>> {
            let stream = task::ready!(Pin::new(&mut self.0.incoming()).poll_next(cx)).unwrap()?;
            Poll::Ready(Some(Ok(stream.compat())))
        }
    }
}

