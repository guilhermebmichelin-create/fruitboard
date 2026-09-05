use serde::Serialize;
use std::fmt::Display;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidRequest,
    Cancelled,
    NotFound,
    Conflict,
    Unavailable,
    Internal,
}

impl ErrorCode {
    fn user_message(self) -> &'static str {
        match self {
            Self::InvalidRequest => "The request was not valid.",
            Self::Cancelled => "The operation was cancelled.",
            Self::NotFound => "The requested item is no longer available.",
            Self::Conflict => "The request could not be completed because its state changed.",
            Self::Unavailable => "The requested service is temporarily unavailable.",
            Self::Internal => "Fruitboard could not complete the request.",
        }
    }

    fn retryable(self) -> bool {
        matches!(self, Self::Conflict | Self::Unavailable)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFacingError {
    pub code: ErrorCode,
    pub message: &'static str,
    pub retryable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticError {
    pub code: ErrorCode,
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppError {
    user: UserFacingError,
    diagnostic: DiagnosticError,
}

impl AppError {
    pub fn new(code: ErrorCode, diagnostic: impl Into<String>) -> Self {
        Self {
            user: UserFacingError {
                code,
                message: code.user_message(),
                retryable: code.retryable(),
            },
            diagnostic: DiagnosticError {
                code,
                summary: diagnostic.into(),
            },
        }
    }

    pub fn invalid_request(diagnostic: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidRequest, diagnostic)
    }

    pub fn unknown(error: impl Display) -> Self {
        Self::new(ErrorCode::Internal, error.to_string())
    }

    pub fn user(&self) -> &UserFacingError {
        &self.user
    }

    pub fn diagnostic(&self) -> &DiagnosticError {
        &self.diagnostic
    }

    pub fn into_user(self) -> UserFacingError {
        self.user
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::test_support::FakeError;
    use serde_json::json;

    #[test]
    fn stable_error_catalog_serializes_exactly() {
        let expected = [
            (
                ErrorCode::InvalidRequest,
                "invalid_request",
                "The request was not valid.",
                false,
            ),
            (
                ErrorCode::Cancelled,
                "cancelled",
                "The operation was cancelled.",
                false,
            ),
            (
                ErrorCode::NotFound,
                "not_found",
                "The requested item is no longer available.",
                false,
            ),
            (
                ErrorCode::Conflict,
                "conflict",
                "The request could not be completed because its state changed.",
                true,
            ),
            (
                ErrorCode::Unavailable,
                "unavailable",
                "The requested service is temporarily unavailable.",
                true,
            ),
            (
                ErrorCode::Internal,
                "internal",
                "Fruitboard could not complete the request.",
                false,
            ),
        ];

        for (code, serialized_code, message, retryable) in expected {
            let error = AppError::new(code, "diagnostic detail");

            assert_eq!(
                serde_json::to_value(error.user()).expect("user error should serialize"),
                json!({
                    "code": serialized_code,
                    "message": message,
                    "retryable": retryable,
                })
            );
        }
    }

    #[test]
    fn user_error_serialization_excludes_diagnostics() {
        let error = AppError::new(
            ErrorCode::Unavailable,
            "C:\\Users\\producer\\Music\\private.flp access_token=secret",
        );

        assert_eq!(
            serde_json::to_value(error.user()).expect("user error should serialize"),
            json!({
                "code": "unavailable",
                "message": "The requested service is temporarily unavailable.",
                "retryable": true,
            })
        );
    }

    #[test]
    fn unknown_errors_map_to_the_stable_internal_code() {
        let error = AppError::unknown(FakeError::new("unexpected adapter failure"));

        assert_eq!(error.user().code, ErrorCode::Internal);
        assert_eq!(
            error.user().message,
            "Fruitboard could not complete the request."
        );
        assert_eq!(error.diagnostic().summary, "unexpected adapter failure");
    }
}
