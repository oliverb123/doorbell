use std::fmt::{Display, Formatter};

use http::Response;
use hyper::Body;

pub mod proxy;
pub mod tracing;

#[derive(Debug)]
pub enum Error {
    Response(Response<Body>),
    Boxed(Box<dyn std::error::Error + Send + Sync>),
    Hyper(hyper::Error),
}

impl From<Response<Body>> for Error {
    fn from(r: Response<Body>) -> Self {
        Error::Response(r)
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for Error {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Error::Boxed(e)
    }
}

impl From<hyper::Error> for Error {
    fn from(e: hyper::Error) -> Self {
        Error::Hyper(e)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Response(r) => write!(f, "Response: {:?}", r),
            Error::Boxed(e) => write!(f, "Boxed: {:?}", e),
            Error::Hyper(e) => write!(f, "Hyper: {:?}", e),
        }
    }
}

impl std::error::Error for Error {}
