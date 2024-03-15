use std::{collections::HashMap, convert::Infallible, time::Duration};

use api::auth::UserInfo;
use hyper::{body::HttpBody, Body, Method, Request, Response};
use tokio::time;

use crate::config::{AppConfig, ConfigWithLock};

pub struct Server;

pub(crate) struct JsonResponse;

impl JsonResponse {
    pub(crate) fn crate_response_map(msg: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("msg".to_string(), msg.to_string());
        map
    }

    pub(crate) fn create_response(status: u16, msg: &str) -> HttpResponse {
        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Body::from(
                serde_json::to_string(&Self::crate_response_map(msg)).unwrap(),
            ))
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
    pub async fn serve(conf: AppConfig<ConfigWithLock>) -> Result<(), hyper::Error> {
        let addr = ([127, 0, 0, 1], 11451).into();
        let conf_inner = conf.clone();
        let make_svc =
            hyper::service::make_service_fn(move |conn: &hyper::server::conn::AddrStream| {
                let conf = conf_inner.clone();
                let remote_addr = conn.remote_addr();
                async move {
                    Ok::<_, hyper::Error>(hyper::service::service_fn(move |req| {
                        println!("{} {} from {}", req.method(), req.uri().path(), remote_addr);
                        Self::router(req, conf.clone())
                    }))
                }
            });
        let server = hyper::Server::bind(&addr).serve(make_svc);
        server
            .with_graceful_shutdown(async move {
                println!("server thread running");
                while conf.running() {
                    time::sleep(Duration::from_millis(250)).await;
                }
                println!("server thread exit");
            })
            .await
    }

    async fn router(
        req: HttpRequest,
        conf: AppConfig<ConfigWithLock>,
    ) -> Result<Response<Body>, Infallible> {
        let path = req.uri().clone();
        match Self::routes(req, conf).await {
            Ok(res) => Ok(res),
            Err(e) => {
                eprintln!("error while processing {}: {}", path, e);
                Ok(JsonResponse::bad_request("Internal Server Error").unwrap())
            }
        }
    }

    async fn routes(req: HttpRequest, conf: AppConfig<ConfigWithLock>) -> HttpResponse {
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/") => Self::handle_index(req, conf).await,
            (&Method::GET, "/user") => Self::handle_get_user_info(req, conf).await,
            (&Method::POST, "/user") => Self::handle_set_user_info(req, conf).await,
            (&Method::GET, "/exit") => Self::handle_exit(req, conf).await,
            _ => Self::handle_not_found(req, conf).await,
        }
    }

    async fn handle_index(_req: Request<Body>, _conf: AppConfig<ConfigWithLock>) -> HttpResponse {
        JsonResponse::ok("hello from htu-net daemon")
    }

    async fn handle_not_found(_req: HttpRequest, _conf: AppConfig<ConfigWithLock>) -> HttpResponse {
        Response::builder()
            .status(404)
            .body(Body::from("Not Found"))
    }

    async fn handle_get_user_info(
        _req: HttpRequest,
        conf: AppConfig<ConfigWithLock>,
    ) -> HttpResponse {
        let conf = conf.config().read().await;
        Ok(Response::new(Body::from({
            let json = serde_json::to_string(&conf.user()).unwrap();
            if json == "null" {
                "{}".to_string()
            } else {
                json
            }
        })))
    }

    async fn handle_set_user_info(
        _req: HttpRequest,
        conf: AppConfig<ConfigWithLock>,
    ) -> HttpResponse {
        let body = match _req.collect().await {
            Ok(b) => b.to_bytes(),
            Err(e) => return JsonResponse::bad_request(&format!("Error reading body: {}", e)),
        };
        let user: UserInfo = match serde_json::from_slice(&body) {
            Ok(info) => info,
            Err(_) => return JsonResponse::bad_request("Invalid user info"),
        };
        conf.config().write().await.set_user(user);
        if let Err(e) = conf.save().await {
            JsonResponse::bad_request(&format!("Error saving conf: {}", e))
        } else {
            JsonResponse::ok("success")
        }
    }

    async fn handle_exit(_req: HttpRequest, mut conf: AppConfig<ConfigWithLock>) -> HttpResponse {
        conf.set_running(false);
        JsonResponse::ok("success")
    }
}
