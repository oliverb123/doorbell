pub mod routes;

use std::net::SocketAddr;

use self::routes::Route;

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
