use http::Request;
use hyper::Body;

use super::rules::JustFilter;

/// A rule which only determines if a received request can be handled by the stack it is in.
pub trait Filter: Send + Sync + 'static {
    type ToMatch;
    /// Check if a given request passes this filter
    fn matches(&self, req: &Self::ToMatch) -> bool;
}

pub fn by_host<S: Into<String>>(host: S) -> JustFilter<impl Filter> {
    let host = host.into();
    (move |req: &Request<Body>| req.uri().host().map(|h| h == host).unwrap_or(false)).into()
}

pub fn by_path<S: Into<String>>(path: S) -> JustFilter<impl Filter> {
    let path = path.into();
    (move |req: &Request<Body>| req.uri().path().starts_with(&path)).into()
}

pub fn has_header<S: Into<String>>(name: S) -> JustFilter<impl Filter> {
    let name = name.into();
    (move |req: &Request<Body>| req.headers().contains_key(&name)).into()
}

pub fn by_header<S: Into<String>, T: Into<String>>(name: S, value: T) -> JustFilter<impl Filter> {
    let name = name.into();
    let value = value.into();
    (move |req: &Request<Body>| {
        req.headers()
            .get(&name)
            .map(|h| h.to_str().map(|s| s == value).unwrap_or(false))
            .unwrap_or(false)
    })
    .into()
}

impl<T> Filter for T
where
    T: Fn(&Self::ToMatch) -> bool + Send + Sync + 'static,
{
    fn matches(&self, req: &Self::ToMatch) -> bool {
        (self)(req)
    }
}
