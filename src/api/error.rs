use gotcha::axum::http::StatusCode;
use gotcha::axum::response::{IntoResponse, Response};
use gotcha::{Json, Schematic};
use serde::Serialize;
use gotcha::oas;
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Schematic)]
pub struct ErrorResponse {
    pub error: String,
}

pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: "Unauthorized".to_string(),
        }
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: msg.into(),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: msg.into(),
        }
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: msg.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(ErrorResponse { error: self.message })).into_response()
    }
}

impl From<crate::TodokiError> for ApiError {
    fn from(e: crate::TodokiError) -> Self {
        Self {
            status: e.to_status_code(),
            message: e.to_string(),
        }
    }
}

impl gotcha::Responsible for ApiError {
    fn response() -> oas::Responses {
        let mut response = oas::Responses {
            default: None,
            data: BTreeMap::default(),
        };
        response.data.insert(
            "4XX".to_string(),
            oas::Referenceable::Data(oas::Response {
                description: "Error response".to_string(),
                headers: None,
                content: Some(BTreeMap::from([(
                    "application/json".to_string(),
                    oas::MediaType {
                        schema: Some(oas::Referenceable::Data(
                            ErrorResponse::generate_schema().schema,
                        )),
                        example: None,
                        examples: None,
                        encoding: None,
                    },
                )])),
                links: None,
            }),
        );
        response
    }
}
