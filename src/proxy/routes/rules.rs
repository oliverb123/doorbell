use std::{ops::Deref, sync::Arc};

use http::Request;
use hyper::Body;

use super::{filter::Filter, map::Map};

pub trait Rule: Send + Sync + 'static {
    type From;
    type To;
    type Error;
    fn matches(&self, req: &Self::From) -> bool;
    fn map(&self, req: Self::From) -> Result<Self::To, Self::Error>;
}

impl<T> Rule for T
where
    T: Filter + Map,
{
    fn matches(&self, req: &Request<Body>) -> bool {
        self.matches(req)
    }

    fn map(&self, req: Request<Body>) -> Result<Request<Body>, Self::Error> {
        self.apply(req)
    }
}

pub struct JustFilter<F: Filter> {
    filter: F,
}

pub struct JustMap<M: Map> {
    map: M,
}

impl<T: Map> Rule for JustMap<T> {
    fn matches(&self, _: &Request<Body>) -> bool {
        true
    }

    fn map(&self, req: Request<Body>) -> Result<Request<Body>, Self::Error> {
        self.map.apply(req)
    }
}

impl<T: Filter> Rule for JustFilter<T> {
    fn matches(&self, req: &Request<Body>) -> bool {
        self.filter.matches(req)
    }

    fn map(&self, req: Request<Body>) -> Result<Request<Body>, Self::Error> {
        Ok(req)
    }
}

impl<T> From<T> for JustMap<T>
where
    T: Map,
{
    fn from(map: T) -> Self {
        Self { map }
    }
}

impl<T> From<T> for JustFilter<T>
where
    T: Filter,
{
    fn from(filter: T) -> Self {
        Self { filter }
    }
}

impl<T> Rule for Arc<T>
where
    T: Rule,
{
    fn matches(&self, req: &Request<Body>) -> bool {
        self.deref().matches(req)
    }

    fn map(&self, req: Request<Body>) -> Result<Request<Body>, Self::Error> {
        self.deref().map(req)
    }
}
