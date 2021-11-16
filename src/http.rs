use std::convert::Infallible;
use axum::{
    body::{Bytes, Full},
    http::{Response, StatusCode},
    response::IntoResponse,
};

pub struct HttpError {
    msg: String,
}

impl IntoResponse for HttpError {
    type Body = Full<Bytes>;
    type BodyError = Infallible;

    fn into_response(self) -> Response<Self::Body> {
        let mut response = self.msg.into_response();
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        response
    }
}

impl<T: std::error::Error> From<T> for HttpError {
    fn from(e: T) -> Self {
        Self {
            msg: format!("{:?}", e),
        }
    }
}

pub type HttpResult<T> = Result<T, HttpError>;
