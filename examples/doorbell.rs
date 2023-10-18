use std::io::BufReader;
use std::net::SocketAddr;

use doorbell::proxy::routes::map::{set_port, set_scheme};
use doorbell::proxy::routes::util::{change_path_prefix, either};
use doorbell::proxy::routes::{filter, make_route, map};
use doorbell::proxy::{forward::ForwardingContext, routes::util::stack};

use doorbell::proxy::Proxy;
use doorbell::tracing::setup_tracing;
use rustls::{Certificate, PrivateKey, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), doorbell::Error> {
    setup_tracing();

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));

    // Some trivial re-writing rules
    let strip_test = stack(filter::by_path("/test/"), map::strip_path("/test/", false));
    let hello_to_world = change_path_prefix("/hello", "/world");

    // This is the upstream we'll send the request to
    let redirect_to_upstream = stack(
        map::set_host("127.0.0.1"),
        map::set_header("host", "127.0.0.1"),
    ) // Setup our host and host header
    .extend(set_port(3000)) // Configure the port
    .extend(set_scheme("http")); // Our upstream doesn't speak https, so we're acting as a TLS terminator

    // Inbound requests usually have a URI that is just a path. This makes use of the Host header to try and
    // resolve the URI to a full URL. Most filters provided in doorbell rely on this resolution. Note that
    // this fails if it can't find the host header, and the uri is partial - you can construct a routing table
    // that only tries to resolve the URI if the host header is present using `filter::has_header`,
    // if you want - for example, if you're serving static content, you probably don't care about having
    // a complete URL, but forwarding requests always does
    let resolve_uri = map::resolve_uri("https");

    // We're a well-behaved proxy, so we try to add forwarding headers to requests we send upstream. The
    // proxy puts the incoming connections `SocketAddr` into the request extensions to support this
    let add_forwarding_headers = map::add_forward_headers("https", 3001);

    // Here, we add a branch to our routing table. The choice of which branch to route a request down is based on
    // calling the `matches` method on each branch in order. The first branch that returns `true` is the one that
    // is used. This means that ordering matters - more specific rules should be earlier in the routing table.
    let rules = either(strip_test, hello_to_world);

    let rules = stack(resolve_uri, rules)
        .extend(redirect_to_upstream)
        .extend(add_forwarding_headers);

    // All routes in the routing table must terminate in a service provider - in fact, the definition of a route is
    // a sequence of rules terminating in a service provider. The service provider is responsible for actually constructing
    // the `hyper::Service` the request is mapped to a response by. The ForwardingContext is just a service provider
    // that shares a hyper::Client across all requests. Anything that implements `MakeService` can be used as a service provider
    let service_provider = ForwardingContext::new();

    let routes = make_route(rules, service_provider);

    let tls_config = get_server_config();

    Proxy::on(addr)
        .https(tls_config)
        .with_routes(routes)
        .serve()
        .await
}

fn get_server_config() -> ServerConfig {
    let certs = get_certs();

    let key = get_key();

    ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .expect("Failed to construct server tls config")
}

fn get_certs() -> Vec<Certificate> {
    let path = std::env::var("DOORBELL_CERT").expect("DOORBELL_CERT not set");
    let file = std::fs::File::open(path).unwrap();
    let mut reader = BufReader::new(file);
    let certs = rustls_pemfile::certs(&mut reader).unwrap();

    certs.into_iter().map(Certificate).collect()
}

fn get_key() -> PrivateKey {
    let path = std::env::var("DOORBELL_PRIV_KEY").expect("DOORBELL_PRIV_KEY not set");
    let file = std::fs::File::open(&path).unwrap();
    let mut reader = BufReader::new(file);
    let mut keys = rustls_pemfile::pkcs8_private_keys(&mut reader).unwrap();

    match keys.len() {
        0 => panic!("No PKCS8-encoded private key found in {path}"),
        1 => PrivateKey(keys.remove(0)),
        _ => panic!("Multiple private keys found in {path}"),
    }
}
