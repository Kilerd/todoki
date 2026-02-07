use gotcha::axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use gotcha::tracing::{debug, warn};

use crate::config::Settings;

/// Authentication context extracted from request
#[derive(Debug, Clone)]
pub enum AuthContext {
    Authenticated,
    None,
}

impl AuthContext {
    pub fn require_auth(&self) -> Result<(), StatusCode> {
        match self {
            AuthContext::Authenticated => Ok(()),
            AuthContext::None => Err(StatusCode::UNAUTHORIZED),
        }
    }
}

/// Simple Bearer token authentication middleware
pub async fn auth_middleware(
    State(settings): State<Settings>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token = auth_header.and_then(|auth| auth.strip_prefix("Bearer ").map(|s| s.to_string()));

    let context = match token {
        Some(t) if t == settings.user_token => {
            debug!("Token authenticated");
            AuthContext::Authenticated
        }
        Some(_) => {
            warn!("Invalid token provided");
            AuthContext::None
        }
        None => {
            debug!("No token provided");
            AuthContext::None
        }
    };

    request.extensions_mut().insert(context);
    Ok(next.run(request).await)
}
