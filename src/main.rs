#![allow(unused)] // For beginning only

use std::fmt::format;
use std::net::SocketAddr;
use axum::response::{Html, IntoResponse};
use axum::{Json, middleware, Router, ServiceExt};
use axum::extract::{Path, Query};
use axum::http::{Method, Uri};
use axum::response::Response;
use axum::routing::{get, get_service};
use serde::Deserialize;
use serde_json::json;
use tower_cookies::CookieManagerLayer;
use tower_http::services::ServeDir;
use uuid::Uuid;
use crate::ctx::Ctx;
use crate::log::log_request;
use crate::model::ModelController;

pub use self::error::{Error, Result};

mod model;
mod web;
mod ctx;
mod error;
mod log;

#[derive(Debug, Deserialize)]
struct HelloParams {
    name: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize application states
    let mc = ModelController::new().await?;

    let api_routes = web::routes_tickets::routes(mc.clone())
        .route_layer(middleware::from_fn(web::mw_auth::mw_require_auth));

    let all_routes = Router::new()
        .merge(hello_routes())
        .merge(web::routes_login::routes())
        .nest("/api", api_routes)
        .layer(axum::middleware::map_response(main_response_mapper))
        .layer(axum::middleware::from_fn_with_state(
            mc.clone(),
            web::mw_auth::mw_ctx_resolver,
        ))
        .layer(CookieManagerLayer::new())
        .fallback_service(static_routes());

    // region: -- start server
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("->> LISTENING on {addr}\n");
    axum::Server::bind(&addr)
        .serve(all_routes.into_make_service())
        .await
        .unwrap();

    // end region: -- start server

    Ok(())
}

async fn main_response_mapper(
    ctx: Option<Ctx>,
    uri: Uri,
    req_method: Method,
    res: Response,
) -> Response {
    println!("->> {:<12} - main_response_mapper", "RES_MAPPER");

    let uuid = Uuid::new_v4();

    // Get the eventual response error
    let service_error = res.extensions().get::<Error>();
    let client_status_error = service_error.map(|err| err.client_status_and_error());

    // If it has an error, build a new response...
    let error_response = client_status_error
        .as_ref()
        .map(|(status_code, client_error)| {
            let client_error_body = json!({
                "error": {
                    "type": client_error.as_ref(),
                    "req_uuid": uuid.to_string(),
                }
            });

            println!("    ->> client_error_body: {client_error_body}");

            // Build the new reponse from the client error body
            (*status_code, Json(client_error_body)).into_response()
        });

    // Build and log the server log line
    let client_error = client_status_error.unzip().1;
    log_request(uuid, req_method, uri, ctx, service_error, client_error).await;

    println!();

    error_response.unwrap_or(res)
}

fn hello_routes() -> Router {
    Router::new()
        .route("/hello", get(hello_handler))
        .route("/hello2/:name", get(hello2_handler))
}

async fn hello_handler(Query(params): Query<HelloParams>) -> impl IntoResponse {
    println!("->> {:<12} - hello_handler - {params:?}", "HANDLER");

    let name = params.name.as_deref().unwrap_or("World");
    Html(format!("Hello <strong>{name}</strong>"))
}

async fn hello2_handler(Path(name): Path<String>) -> impl IntoResponse {
    println!("->> {:<12} - hello2_handler - {name:?}", "HANDLER");
    Html(format!("Hello2 <strong>{name}</strong>"))
}

fn static_routes() -> Router {
    Router::new().nest_service("/", get_service(ServeDir::new("./public/")))
}
