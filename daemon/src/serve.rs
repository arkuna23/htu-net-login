use std::convert::Infallible;

use hyper::{Body, Request, Response};

pub struct Server;

impl Server {
    async fn router(req: Request<Body>) -> Result<Response<Body>, Infallible> {
        match (req.method(), req.uri().path()) {}
    }
}
