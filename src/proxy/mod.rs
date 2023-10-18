pub mod forward;
pub mod routes;

use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use futures_util::StreamExt;
use hyper::{
    server::{
        accept,
        conn::{AddrIncoming, AddrStream},
    },
    service::{make_service_fn, service_fn},
    Server,
};
use tls_listener::TlsListener;
use tokio_rustls::server::TlsStream;
use tracing::warn;

use crate::Error;

use self::{forward::no_match, routes::Route};

pub struct Proxy<State> {
    on: SocketAddr,
    state: State,
}

pub struct NeedsProtocol;

pub struct Http;

pub struct Https {
    tls: rustls::ServerConfig,
}

pub struct NeedsRules<Proto> {
    proto: Proto,
}

pub struct CanServe<Proto, R: Route> {
    proto: Proto,
    routes: R,
}

impl Proxy<NeedsProtocol> {
    pub fn on(addr: SocketAddr) -> Self {
        Self {
            on: addr,
            state: NeedsProtocol,
        }
    }

    pub fn http(self) -> Proxy<NeedsRules<Http>> {
        Proxy {
            on: self.on,
            state: NeedsRules { proto: Http },
        }
    }

    pub fn https(self, tls: rustls::ServerConfig) -> Proxy<NeedsRules<Https>> {
        Proxy {
            on: self.on,
            state: NeedsRules {
                proto: Https { tls: tls },
            },
        }
    }
}

impl<Proto> Proxy<NeedsRules<Proto>> {
    pub fn with_routes<R: Route>(self, routes: R) -> Proxy<CanServe<Proto, R>> {
        Proxy {
            on: self.on,
            state: CanServe {
                proto: self.state.proto,
                routes,
            },
        }
    }
}

impl<Proto, R: Route> Proxy<CanServe<Proto, R>> {
    fn configure_incoming(&self, mut incoming: AddrIncoming) -> AddrIncoming {
        incoming.set_nodelay(true);
        incoming
    }
}

impl<R: Route> Proxy<CanServe<Http, R>> {
    pub async fn serve(self) -> Result<(), Error> {
        let accept = self.configure_incoming(AddrIncoming::bind(&self.on)?);

        let routes = Arc::new(self.state.routes);

        let make_service = make_service_fn(move |conn: &AddrStream| {
            let downstream = conn.remote_addr();
            let routes = routes.clone();

            async move {
                Ok::<_, Infallible>(service_fn(move |mut req| {
                    req.extensions_mut().insert(downstream.clone());
                    let routes = routes.clone();
                    async move {
                        if !routes.matches(&req) {
                            Ok(no_match("no matching route"))
                        } else {
                            let (req, mut service) = routes.route(req)?;
                            service.call(req).await
                        }
                    }
                }))
            }
        });

        let server = Server::builder(accept).http1_only(true).serve(make_service);
        Ok(server.await?)
    }
}

impl<R: Route> Proxy<CanServe<Https, R>> {
    pub async fn serve(self) -> Result<(), Error> {
        let incoming = self.configure_incoming(AddrIncoming::bind(&self.on)?);

        let tls = Arc::new(self.state.proto.tls);
        let acceptor = tokio_rustls::TlsAcceptor::from(tls);

        let listener = TlsListener::new(acceptor, incoming).filter(|conn| {
            if let Err(err) = conn {
                warn!("Error establishing TLS connection: {:?}", err);
                std::future::ready(false)
            } else {
                std::future::ready(true)
            }
        });

        let routes = Arc::new(self.state.routes);

        let make_service = make_service_fn(move |conn: &TlsStream<AddrStream>| {
            let downstream = conn.get_ref().0.remote_addr();
            let routes = routes.clone();

            async move {
                Ok::<_, Infallible>(service_fn(move |mut req| {
                    req.extensions_mut().insert(downstream.clone());
                    let routes = routes.clone();
                    async move {
                        if !routes.matches(&req) {
                            Ok(no_match("no matching route"))
                        } else {
                            let (req, mut service) = routes.route(req)?;
                            service.call(req).await
                        }
                    }
                }))
            }
        });

        let server = Server::builder(accept::from_stream(listener))
            .http1_only(true)
            .serve(make_service);

        Ok(server.await?)
    }
}
