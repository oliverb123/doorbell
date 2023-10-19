use std::{ops::Deref, sync::Arc};

use crate::Error;

/// A simple mapping from one request to another. Should be used in combination
/// with filters to construct a routing table
pub trait Map: Send + Sync + 'static {
    type From;
    type To;
    type Error;
    /// Check if a given request passes this filter
    fn apply(&self, req: Self::From) -> Result<Self::To, Self::Error>;
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
