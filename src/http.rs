use axum::{
    body::{Bytes, Full},
    http::{Response, StatusCode},
    response::IntoResponse,
};
use std::{convert::Infallible, fmt::Display};

#[derive(PartialEq, Debug)]
pub struct HttpError {
    pub message: String,
    pub status_code: StatusCode,
}

impl IntoResponse for HttpError {
    type Body = Full<Bytes>;
    type BodyError = Infallible;

    fn into_response(self) -> Response<Self::Body> {
        let mut response = self.message.into_response();
        *response.status_mut() = self.status_code;
        response
    }
}

impl<T: std::error::Error> From<T> for HttpError {
    fn from(e: T) -> Self {
        Self {
            message: format!("{:?}", e),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[macro_export]
macro_rules! http_err {
    ($status: path, $fmt: literal, $($args: tt)+) => {
        HttpError {
            message: format!($fmt, $($args)+),
            status_code: $status
        }
    };
    ($status: path, $msg: literal) => {
        HttpError {
            message: $msg.to_string(),
            status_code: $status
        }
    };
    ($fmt: literal, $($args: tt)+) => {
        http_err!(StatusCode::INTERNAL_SERVER_ERROR, $fmt, $($args)+)
    };
    ($msg: literal) => {
        http_err!(StatusCode::INTERNAL_SERVER_ERROR, $msg)
    };
}

#[macro_export]
macro_rules! http_bail {
    ($($args: tt)+) => {
        return Err(http_err!($($args)+));
    };
}

pub trait HttpContext<T> {
    fn http_context<C>(self, status_code: StatusCode, extra_msg: C) -> Result<T, HttpError>
    where
        C: Display + Send + Sync + 'static;

    fn http_error<C>(self, extra_msg: C) -> Result<T, HttpError>
    where
        C: Display + Send + Sync + 'static;
}

impl<T, E> HttpContext<T> for Result<T, E>
where
    E: std::fmt::Debug + Sync + Send + 'static,
{
    fn http_context<C>(self, status_code: StatusCode, extra_msg: C) -> Result<T, HttpError>
    where
        C: Display + Send + Sync + 'static,
    {
        self.map_err(|e| HttpError {
            message: format!("{}: {:?}", extra_msg, e),
            status_code,
        })
    }

    fn http_error<C>(self, extra_msg: C) -> Result<T, HttpError>
    where
        C: Display + Send + Sync + 'static,
    {
        self.map_err(|e| HttpError {
            message: format!("{}: {:?}", extra_msg, e),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })
    }
}

pub type HttpResult<T> = Result<T, HttpError>;

#[cfg(test)]
mod test {
    use crate::HttpError;
    use axum::http::StatusCode;

    #[test]
    fn test_macros() {
        let error = HttpError {
            message: "aaa".to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        };
        assert_eq!(error, http_err!(StatusCode::INTERNAL_SERVER_ERROR, "aaa"));
        assert_eq!(
            error,
            http_err!(StatusCode::INTERNAL_SERVER_ERROR, "{}aa", "a")
        );
        assert_eq!(error, http_err!("aaa"));
        assert_eq!(error, http_err!("{}aa", "a"));
    }
}
