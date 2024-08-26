use std::sync::Arc;

use axum::{
    extract::{FromRef, State},
    http::StatusCode,
};

use super::AppDeps;

#[derive(Debug, Clone)]
pub struct AuthDeps {
    pub client_secret: String,
}

impl FromRef<Arc<AppDeps>> for AuthDeps {
    fn from_ref(input: &Arc<AppDeps>) -> Self {
        Self {
            client_secret: input.client_secret.clone(),
        }
    }
}

pub async fn auth(
    State(auth_deps): State<AuthDeps>,
    axum_extra::typed_header::TypedHeader(headers::Authorization(bearer)): axum_extra::typed_header::TypedHeader<
        headers::Authorization<headers::authorization::Bearer>,
    >,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    if auth_deps.client_secret != bearer.token() {
        return Err(StatusCode::FORBIDDEN);
    }
    let response = next.run(request).await;
    Ok(response)
}
