use std::pin::Pin;

use hyper::{client::HttpConnector, service::service_fn, Body, Client, Request, Response};
use std::future::Future;

use crate::Error;

use super::routes::services::{MakeService, RouteService};

pub struct ForwardingContext {
    client: Client<HttpConnector, Body>,
}

impl ForwardingContext {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    async fn forward_request(
        client: Client<HttpConnector, Body>,
        req: Request<Body>,
    ) -> Result<Response<Body>, Error> {
        let res = client.request(req).await;

        let res = map_connection_refused_to_bad_gateway(res);

        Ok(res?)
    }
}

impl MakeService for ForwardingContext {
    fn make(&self) -> Box<dyn RouteService<Request<Body>, Response<Body>, crate::Error>> {
        let client = self.client.clone();
        Box::new(service_fn(move |req| {
            let client = client.clone();
            Box::pin(Self::forward_request(client, req))
                as Pin<Box<dyn Future<Output = _> + Send + 'static>>
        }))
    }
}

// TODO - make this configurable
fn map_connection_refused_to_bad_gateway(
    res: Result<Response<Body>, hyper::Error>,
) -> Result<Response<Body>, hyper::Error> {
    match res {
        Ok(res) => Ok(res),
        Err(err) => {
            if err.is_connect() {
                Ok(Response::builder()
                    .status(502)
                    .body(Body::from("Bad Gateway\n"))
                    .unwrap())
            } else {
                Err(err)
            }
        }
    }
}

pub(crate) fn no_match<R: Into<String>>(reason: R) -> Response<Body> {
    Response::builder()
        .status(404)
        .body(Body::from(format!("No match: {}\n", reason.into())))
        .unwrap()
}
