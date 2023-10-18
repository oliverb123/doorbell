use std::sync::Arc;

use self::{
    rules::Rule,
    services::{MakeService, RouteService},
};

pub mod filter;
pub mod map;
pub mod rules;
pub mod services;
pub mod util;

pub trait Route: Send + Sync + 'static {
    type Request;
    type Mapped;
    type Response;
    type Error;
    fn matches(&self, req: &Self::Request) -> bool;

    fn route(
        &self,
        req: Self::Request,
    ) -> Result<
        (
            Self::Mapped,
            Box<dyn RouteService<Self::Request, Self::Response, Self::Error>>,
        ),
        Self::Error,
    >;
}

pub struct Routed<R: Rule, I: MakeService> {
    rule: R,
    inner: I,
}

pub fn make_route<R: Rule, I: MakeService>(rule: R, inner: I) -> Routed<R, I> {
    Routed { rule, inner }
}

impl<T> Route for Arc<T>
where
    T: Route + Sized,
{
    fn matches(&self, req: &T::Request) -> bool {
        (self as &T).matches(req)
    }

    fn route(
        &self,
        req: T::Request,
    ) -> Result<
        (
            T::Request,
            Box<dyn RouteService<T::Request, T::Response, T::Error>>,
        ),
        T::Error,
    > {
        (self as &T).route(req)
    }
}

impl<R, S, Request, Mapped, Response, Error> Route for Routed<R, S>
where
    R: Rule<From = Request, To = Mapped, Error = Error>,
    S: MakeService<Request = Mapped, Response = Response, Error = Error>,
{
    fn matches(&self, req: &Request) -> bool {
        self.rule.matches(req)
    }

    fn route(
        &self,
        req: Request,
    ) -> Result<(Mapped, Box<dyn RouteService<Mapped, Response, Error>>), Error> {
        let req = self.rule.map(req);
        let req = req?;
        Ok((req, (self.inner).call()))
    }
}
