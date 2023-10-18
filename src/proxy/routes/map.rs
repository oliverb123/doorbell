use std::{net::SocketAddr, ops::Deref, sync::Arc};

use http::{HeaderName, Request};
use url::Url;

use crate::{proxy::forward::no_match, Error};

use super::rules::JustMap;

/// A simple mapping from one request to another. Should be used in combination
/// with filters to construct a routing table
pub trait Map: Send + Sync + 'static {
    type From;
    type To;
    type Error;
    /// Check if a given request passes this filter
    fn apply(&self, req: Self::From) -> Result<Self::To, Self::Error>;
}

pub fn set_port<Body>(port: u16) -> JustMap<impl Map> {
    (move |mut req: Request<Body>| {
        let Ok(mut url) = req.uri().to_string().parse::<Url>() else {
            return Err(no_match("SetPort: invalid uri").into());
        };
        if let Err(_) = url.set_port(Some(port)) {
            return Err(no_match("SetPort: failed to set port").into());
        }
        let Ok(uri) = url.as_str().parse() else {
            return Err(no_match("SetPort: invalid uri after rewrite").into());
        };
        *req.uri_mut() = uri;
        Ok(req)
    })
    .into()
}

pub fn strip_path<S: Into<String>, Body>(prefix: S, permissive: bool) -> JustMap<impl Map> {
    let prefix = prefix.into();
    (move |mut req: Request<Body>| {
        let Ok(mut url) = req.uri().to_string().parse::<Url>() else {
            return Err(no_match("StripPath: invalid uri").into());
        };

        let path = url.path();
        let Some(path) = path.strip_prefix(&prefix) else {
            return return_early(req, permissive, "StripPath: path doesn't match prefix");
        };

        url.set_path(&format!("{}", path));

        let Ok(uri) = url.as_str().parse() else {
            return Err(no_match("RewritePath: invalid uri after rewrite").into());
        };
        *req.uri_mut() = uri;
        Ok(req)
    })
    .into()
}

pub fn add_prefix<S: Into<String>, Body>(prefix: S) -> JustMap<impl Map> {
    let prefix = prefix.into();
    (move |mut req: Request<Body>| {
        let Ok(mut url) = req.uri().to_string().parse::<Url>() else {
            return Err(no_match("AddPrefix: invalid uri").into());
        };

        let path = url.path();
        url.set_path(&format!("{}{}", prefix, path));

        let Ok(uri) = url.as_str().parse() else {
            return Err(no_match("AddPrefix: invalid uri after rewrite").into());
        };
        *req.uri_mut() = uri;
        Ok(req)
    })
    .into()
}

pub fn set_host<S: Into<String>, Body>(to: S) -> JustMap<impl Map> {
    let to = to.into();
    (move |mut req: Request<Body>| {
        let Ok(mut url) = req.uri().to_string().parse::<Url>() else {
            return Err(no_match("SetHost: invalid uri").into());
        };

        if let Err(_) = url.set_host(Some(&to)) {
            return Err(no_match("SetHost: failed to set host").into());
        }

        let Ok(uri) = url.as_str().parse() else {
            return Err(no_match("SetHost: invalid uri after rewrite").into());
        };
        *req.uri_mut() = uri;
        Ok(req)
    })
    .into()
}

pub fn set_header<S: Into<String>, Body>(name: S, value: S) -> JustMap<impl Map> {
    let name = name.into();
    let value = value.into();
    (move |mut req: Request<Body>| {
        req.headers_mut()
            .insert(name.parse::<HeaderName>().unwrap(), value.parse().unwrap());
        Ok(req)
    })
    .into()
}

pub fn add_header<S: Into<String>, Body>(name: S, value: S) -> JustMap<impl Map> {
    let name = name.into();
    let value = value.into();
    (move |mut req: Request<Body>| {
        req.headers_mut()
            .append(name.parse::<HeaderName>().unwrap(), value.parse().unwrap());
        Ok(req)
    })
    .into()
}

pub fn resolve_uri<S: Into<String>, Body>(proto: S) -> JustMap<impl Map> {
    let proto = proto.into();
    let map = move |req: Request<Body>| {
        if req.uri().authority().is_some() && req.uri().scheme().is_some() {
            return Ok(req);
        }

        // Handle the protocol
        let mut url_string = String::with_capacity(255);
        url_string.push_str(&proto);
        url_string.push_str("://");

        // Handle the host/port, returning no_match if we've been called in error
        let host_port = match req.headers().get("host").as_ref().map(|r| r.to_str()) {
            Some(Ok(host_port)) => host_port,
            _ => return Err(no_match("ResolveUri: no host header").into()),
        };
        url_string.push_str(host_port);

        // Handle the rest of the url - path, and query. http::Uri doesn't include fragments, because it's not
        // a compliant URI parser, so we, uh, don't handle fragments.
        let mut url = Url::parse(&url_string).unwrap();
        url.set_path(req.uri().path());
        url.set_query(req.uri().query());

        let mut req = req;
        let Ok(uri) = url.as_str().parse() else {
            return Err(no_match(format!("ResolveUri: invalid uri after rewrite: {}", url)).into());
        };
        *req.uri_mut() = uri;
        Ok(req)
    };
    map.into()
}

pub fn add_forward_headers<S: Into<String>, Body>(proto: S, port: u16) -> JustMap<impl Map> {
    let proto = proto.into();
    (move |req: Request<Body>| {
        let (mut parts, body) = req.into_parts();
        let host = parts.uri.host();

        let downstream = parts.extensions.get::<SocketAddr>();

        let headers = &mut parts.headers;

        if let Some(downstream) = downstream {
            headers.insert(
                "x-forwarded-for",
                downstream.ip().to_string().parse().unwrap(),
            );
        }

        headers.insert("x-forwarded-proto", proto.parse().unwrap());

        headers.insert("x-forwarded-port", port.into());

        if let Some(host) = headers.get("host") {
            headers.insert("x-forwarded-host", host.clone());
        }

        // If the URI host and the header host both exist, we trust the uri more
        if let Some(host) = host {
            headers.insert("x-forwarded-host", host.parse().unwrap());
        }

        Ok(Request::from_parts(parts, body))
    })
    .into()
}

pub fn set_scheme<S: Into<String>, Body>(scheme: S) -> JustMap<impl Map> {
    let scheme = scheme.into();
    (move |mut req: Request<Body>| {
        let Ok(mut url) = req.uri().to_string().parse::<Url>() else {
            return Err(no_match("SetScheme: invalid uri").into());
        };

        if let Err(_) = url.set_scheme(&scheme) {
            return Err(no_match("SetScheme: failed to set scheme").into());
        }

        let Ok(uri) = url.as_str().parse() else {
            return Err(no_match("SetScheme: invalid uri after rewrite").into());
        };
        *req.uri_mut() = uri;
        Ok(req)
    })
    .into()
}

impl<T> Map for Arc<T>
where
    T: Map,
{
    fn apply(&self, req: T::From) -> Result<T::To, T::Error> {
        self.deref().apply(req)
    }
}

impl<T> Map for T
where
    T: Fn(Self::From) -> Result<Self::To, Self::Error> + Send + Sync + 'static,
{
    fn apply(&self, req: Self::From) -> Result<Self::To, Self::Error> {
        (self)(req)
    }
}

pub fn return_early<S: Into<String>, From, To>(
    req: From,
    permissive: bool,
    reason: S,
) -> Result<To, Error> {
    if permissive {
        Ok(req)
    } else {
        Err(no_match(reason).into())
    }
}
