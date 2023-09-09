use async_trait::async_trait;
use axum::extract::{FromRequestParts, State};
use axum::http::Request;
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::RequestPartsExt;
use axum::response::Response;
use lazy_regex::regex_captures;
use tower_cookies::{Cookie, Cookies};
use crate::{Error, Result};
use crate::ctx::Ctx;
use crate::model::ModelController;
use crate::web::AUTH_TOKEN;

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Ctx {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self> {
        println!("->> {:<12} - Ctx", "EXTRACTOR");

        parts.extensions
            .get::<Result<Ctx>>()
            .ok_or(Error::AuthFailCtxNotInRequestExt)?
            .clone()
    }
}

pub async fn mw_require_auth<B>(
    ctx: Result<Ctx>,
    req: Request<B>,
    next: Next<B>
) -> Result<Response> {
    println!("->> {:<12} - mw_require_auth", "MIDDLEWARE");

    ctx?;

    Ok(next.run(req).await)
}

pub async fn mw_ctx_resolver<B>(
    _mc: State<ModelController>,
    cookies: Cookies,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response> {
    println!("->> {:<12} - mw_ctx_resolver", "MIDDLEWARE");

    let auth_token = cookies.get(AUTH_TOKEN).map(|c| c.value().to_string());

    // Compute Result<Ctx>
    let result_ctx = match auth_token
        .ok_or(Error::AuthFailNoAuthCookie)
        .and_then(parse_token)
    {
        Ok((user_id, _exp, _sign)) => {
            Ok(Ctx::new(user_id))
        },
        Err(e) => Err(e),
    };

    if result_ctx.is_err() && !matches!(result_ctx, Err(Error::AuthFailNoAuthCookie)) {
        cookies.remove(Cookie::named(AUTH_TOKEN))
    }

    // Store the ctx_result in the request extension
    req.extensions_mut().insert(result_ctx);

    Ok(next.run(req).await)
}

fn parse_token(token: String) -> Result<(u64, String, String)> {
    let (_whole, user_id, exp, sign) = regex_captures!(
        r#"^user-(\d+)\.(.+)\.(.+)"#,
        &token
    ).ok_or(Error::AuthFailTokenWrongFormat)?;

    let user_id: u64 = user_id.parse().map_err(|_| Error::AuthFailTokenWrongFormat)?;

    Ok((user_id, exp.to_string(), sign.to_string()))
}