use std::{future::Future, pin::Pin};

pub trait MakeService: Send + Sync + 'static {
    type Request;
    type Response;
    type Error;
    fn make(&self) -> Box<dyn RouteService<Self::Request, Self::Response, Self::Error>>;
}

// Map from a single request to a single response
pub trait RouteService<Request, Response, Error> {
    fn call(&mut self, req: Request) -> RouteFut<Response, Error>;
}

pub type RouteFut<Response, Error> =
    Pin<Box<dyn Future<Output = Result<Response, Error>> + Send + 'static>>;

impl<F> MakeService for F
where
    F: Fn() -> Box<dyn RouteService<Self::Request, Self::Response, Self::Error>>
        + Send
        + Sync
        + 'static,
{
    fn make(&self) -> Box<dyn RouteService<Self::Request, Self::Response, Self::Error>> {
        (self)()
    }
}

impl<F, Request, Response, Error> RouteService<Request, Response, Error> for F
where
    F: FnMut(Request) -> RouteFut<Response, Error> + Send + Sync + 'static,
{
    fn call(&mut self, req: Request) -> RouteFut<Response, Error> {
        (self)(req)
    }
}
