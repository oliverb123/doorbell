pub mod proxy;

#[derive(Debug)]
pub enum Error {
    Boxed(Box<dyn std::error::Error + Send + Sync>),
}
