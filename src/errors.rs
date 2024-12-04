use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub struct WeatherVaneError(pub anyhow::Error);

impl<E> From<E> for WeatherVaneError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

impl IntoResponse for WeatherVaneError {
    fn into_response(self) -> Response {
        {
            tracing::error!(error = ?self.0, "internal server error");
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}
