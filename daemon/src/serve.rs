use std::{collections::HashMap, convert::Infallible};

use hyper::{Body, Method, Request, Response};
use tokio::sync::mpsc::Sender;

use crate::{config::{ConfigFile, ConfigWithLock}, daemon::Signal};

pub struct Server;

pub(crate) struct JsonResponse;

impl JsonResponse {
    pub(crate) fn crate_response_map(msg: &str) -> HashMap<String, String> {
        HashMap::new();
        map.insert("msg", msg);
        map
    }

    pub(crate) fn create_response(status:u16, msg: &str) -> HttpResponse {
        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Self::crate_response_map(msg)).unwrap()
    }

    pub(crate) fn ok(msg: &str) -> HttpResponse {
        Self::create_response(200, msg)
    }

    pub(crate) fn bad_request(msg: &str) -> HttpResponse {
        Self::create_response(400, msg)
    }
}

type HttpResponse = Result<Response<Body>, hyper::http::Error>;
type HttpRequest = Request<Body>;
impl Server {
    pub async fn serve(conf: ConfigFile<ConfigWithLock>) -> Result<(), hyper::Error> {
        let addr = ([127, 0, 0, 1], 1145).into();
        let make_svc = hyper::service::make_service_fn(move |_conn| {
            let conf = conf.clone();
            async {
                Ok::<_, hyper::Error>(hyper::service::service_fn(move |req| {
                    Self::router(req, conf);
                }))
            }
        });
        let server = hyper::Server::bind(&addr).serve(make_svc);
        server.await
    }

    async fn router(req: HttpRequest, conf: ConfigFile<ConfigWithLock>) -> Result<Response<Body>, Infallible> {
        let path = req.uri().clone();
        match Self::routes(req, conf).await {
            Ok(res) => Ok(res),
            Err(e) => {
                eprintln!("error while processing {}: {}", path, e);
                Ok(JsonResponse::bad_request("Internal Server Error"))
            },
        }
    }

    async fn routes(req: HttpRequest, conf: ConfigFile<ConfigWithLock>) -> HttpResponse {
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/") => Self::handle_index(req, tx).await,
            _ => Self::handle_not_found(req, conf).await,
        }
    }

    async fn handle_index(_req: Request<Body>, conf: ConfigFile<ConfigWithLock>) -> HttpResponse {
        Ok(JsonResponse::ok("hello from htu-net daemon"))
    }

    async fn handle_not_found(_req: HttpRequest, conf: ConfigFile<ConfigWithLock>) -> HttpResponse {
        Response::builder()
            .status(404)
            .body(Body::from("Not Found"))
    }

    async fn handle_get_user_info(_req: HttpRequest, conf: ConfigFile<ConfigWithLock>) -> HttpResponse {
        let conf = conf.lock().await;
        Ok(Response::new(Body::from(serde_json::to_string(&conf.user()).unwrap())))
    }

    async fn handle_set_user_info(_req: HttpRequest, conf: ConfigFile<ConfigWithLock>) -> HttpResponse {
        let body = hyper::body::to_bytes(_req.into_body()).await?;
        let user: UserInfo = match serde_json::from_slice(&body) {
            Ok(info) => info,
            Err(_) => JsonResponse::bad_request("Invalid user info"),
        };
        let mut conf = conf.lock().await;
        conf.set_user(user);
        Ok(JsonResponse::ok("success"))
    }

}
