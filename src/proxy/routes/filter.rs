/// A rule which only determines if a received request can be handled by the stack it is in.
pub trait Filter: Send + Sync + 'static {
    type ToMatch;
    /// Check if a given request passes this filter
    fn matches(&self, req: &Self::ToMatch) -> bool;
}

impl<T> Filter for T
where
    T: Fn(&Self::ToMatch) -> bool + Send + Sync + 'static,
{
    fn matches(&self, req: &Self::ToMatch) -> bool {
        (self)(req)
    }
}
