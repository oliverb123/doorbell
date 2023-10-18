use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Duration;

use doorbell::tracing::setup_tracing;
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::{Body, Request, Response};
use tokio::net::TcpListener;
use tracing::{info, warn};

async fn hello(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    info!(req = ?req, "Got a request");
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok(Response::new(Body::from(format!("{:?}\n", req))))
}

#[tokio::main]
async fn main() {
    setup_tracing();
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(addr).await.unwrap();

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await.unwrap();

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = Http::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(stream, service_fn(hello))
                .await
            {
                warn!("Error serving connection: {:?}", err);
            }
        });
    }
}
