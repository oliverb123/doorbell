use http::Request;
use hyper::Body;

use crate::Error;

use super::{filter, map, rules::Rule, services::RouteService, Route};

pub fn change_path_prefix<S: Into<String>>(from: S, to: S) -> impl Rule {
    let from = from.into();
    let map = stack(
        map::strip_path(from.clone(), false),
        map::add_prefix(to.into()),
    );
    stack(filter::by_path(from), map)
}

pub fn stack<A, B>(first: A, second: B) -> Stack<A, B> {
    Stack {
        a: first,
        b: second,
    }
}

pub fn either<A, B>(a: A, b: B) -> Alternatives<A, B> {
    Alternatives { a, b }
}

pub struct Alternatives<A, B> {
    a: A,
    b: B,
}

impl<A, B> Alternatives<A, B> {
    /// Add a rule before this one - alternative rules are greedy, so ordering matters
    pub fn push_front<O>(self, other: O) -> Alternatives<O, Self> {
        either(other, self)
    }

    /// Add a rule after this one - alternative rules are greedy, so ordering matters
    pub fn push_back<O>(self, other: O) -> Alternatives<Self, O> {
        either(self, other)
    }
}

impl<A, B, Request, Mapped, Response, Error> Route for Alternatives<A, B>
where
    A: Route<Request = Request, Mapped = Mapped, Response = Response, Error = Error>,
    B: Route<Request = Request, Mapped = Mapped, Response = Response, Error = Error>,
{
    fn matches(&self, req: &Request) -> bool {
        let res = self.a.matches(req) || self.b.matches(req);
        res
    }

    fn route(
        &self,
        req: Request,
    ) -> Result<(Mapped, Box<dyn RouteService<Mapped, Response, Error>>), Error> {
        if self.a.matches(&req) {
            self.a.route(req)
        } else {
            self.b.route(req)
        }
    }
}

impl<A, B> Rule for Alternatives<A, B>
where
    A: Rule,
    B: Rule,
{
    fn matches(&self, req: &Request<Body>) -> bool {
        self.a.matches(req) || self.b.matches(req)
    }

    fn map(&self, req: Request<Body>) -> Result<Request<Body>, Error> {
        if self.a.matches(&req) {
            self.a.map(req)
        } else {
            self.b.map(req)
        }
    }
}

pub struct Stack<A, B> {
    a: A,
    b: B,
}

impl<A, B> Stack<A, B> {
    pub fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

impl<A: Rule, B: Rule> Stack<A, B> {
    pub fn extend(self, other: impl Rule) -> Stack<Self, impl Rule> {
        Stack { a: self, b: other }
    }

    pub fn push_front(self, other: impl Rule) -> Stack<impl Rule, Self> {
        Stack { a: other, b: self }
    }
}

impl<A: Rule, B: Rule> Rule for Stack<A, B> {
    fn matches(&self, req: &Request<Body>) -> bool {
        self.a.matches(req) && self.b.matches(req)
    }

    fn map(&self, req: Request<Body>) -> Result<Request<Body>, Error> {
        self.b.map(self.a.map(req)?)
    }
}
