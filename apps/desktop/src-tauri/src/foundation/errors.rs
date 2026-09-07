use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidRequest,
    Cancelled,
    NotFound,
    Conflict,
    Unavailable,
    Internal,
    /// Scan/Library console: the location could not be accessed.
    #[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
    AccessDenied,
    /// Scan/Library console: the operation is not supported for this location.
    #[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
    Unsupported,
    /// Scan/Library console: the operation exceeded a resource limit.
    #[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
    ResourceLimit,
    /// Library pages: the committed snapshot changed since the cursor.
    #[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
    StaleCursor,
    /// Library pages: the cursor or snapshot token is malformed or wrong-root.
    #[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
    InvalidCursor,
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
            Self::AccessDenied => "The requested location could not be accessed.",
            Self::Unsupported => "The requested operation is not supported for this location.",
            Self::ResourceLimit => "The operation exceeded a resource limit.",
            Self::StaleCursor => "The list changed; start again from the first page.",
            Self::InvalidCursor => "The page continuation is not valid; start from the first page.",
        }
    }

    fn retryable(self) -> bool {
        matches!(self, Self::Conflict | Self::Unavailable)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DiagnosticCode {
    CommandPanicked,
    RequestSchemaValidationFailed,
    StorageBusy,
    StorageFailed,
    ScanRootConflict,
    UnknownScanRoot,
    /// The scan console is compiled out (feature disabled); every console
    /// command returns the fixed unavailable envelope.
    #[cfg_attr(feature = "scan-console", allow(dead_code))]
    ScanConsoleDisabled,
    /// The scan request conflicted with durable queue state.
    ScanJobConflict,
    /// A cancel/retry referenced an unknown job.
    UnknownScanJob,
    /// A library cursor/snapshot token is malformed or wrong-root.
    InvalidLibraryCursor,
    /// A library cursor/snapshot refers to a replaced committed snapshot.
    StaleLibraryCursor,
    #[allow(
        dead_code,
        reason = "reserved for adapters added after the command foundation"
    )]
    UnexpectedFailure,
}

impl DiagnosticCode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::CommandPanicked => "command_panicked",
            Self::RequestSchemaValidationFailed => "request_schema_validation_failed",
            Self::StorageBusy => "storage_busy",
            Self::StorageFailed => "storage_failed",
            Self::ScanRootConflict => "scan_root_conflict",
            Self::UnknownScanRoot => "unknown_scan_root",
            Self::ScanConsoleDisabled => "scan_console_disabled",
            Self::ScanJobConflict => "scan_job_conflict",
            Self::UnknownScanJob => "unknown_scan_job",
            Self::InvalidLibraryCursor => "invalid_library_cursor",
            Self::StaleLibraryCursor => "stale_library_cursor",
            Self::UnexpectedFailure => "unexpected_failure",
        }
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
    pub diagnostic_code: DiagnosticCode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppError {
    user: UserFacingError,
    diagnostic: DiagnosticError,
}

impl AppError {
    pub(crate) fn new(code: ErrorCode, diagnostic_code: DiagnosticCode) -> Self {
        Self {
            user: UserFacingError {
                code,
                message: code.user_message(),
                retryable: code.retryable(),
            },
            diagnostic: DiagnosticError {
                code,
                diagnostic_code,
            },
        }
    }

    pub(crate) fn invalid_request(diagnostic_code: DiagnosticCode) -> Self {
        Self::new(ErrorCode::InvalidRequest, diagnostic_code)
    }

    #[allow(
        dead_code,
        reason = "the first fallible product adapter lands after this foundation"
    )]
    pub(crate) fn unknown<Source>(_source: Source) -> Self {
        Self::new(ErrorCode::Internal, DiagnosticCode::UnexpectedFailure)
    }

    pub(crate) fn command_panicked() -> Self {
        Self::new(ErrorCode::Internal, DiagnosticCode::CommandPanicked)
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
            (
                ErrorCode::AccessDenied,
                "access_denied",
                "The requested location could not be accessed.",
                false,
            ),
            (
                ErrorCode::Unsupported,
                "unsupported",
                "The requested operation is not supported for this location.",
                false,
            ),
            (
                ErrorCode::ResourceLimit,
                "resource_limit",
                "The operation exceeded a resource limit.",
                false,
            ),
            (
                ErrorCode::StaleCursor,
                "stale_cursor",
                "The list changed; start again from the first page.",
                false,
            ),
            (
                ErrorCode::InvalidCursor,
                "invalid_cursor",
                "The page continuation is not valid; start from the first page.",
                false,
            ),
        ];

        for (code, serialized_code, message, retryable) in expected {
            let error = AppError::new(code, DiagnosticCode::UnexpectedFailure);

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
        let error = AppError::new(ErrorCode::Unavailable, DiagnosticCode::UnexpectedFailure);

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
        assert_eq!(
            error.diagnostic().diagnostic_code,
            DiagnosticCode::UnexpectedFailure
        );
        assert_ne!(
            error.diagnostic().diagnostic_code.as_str(),
            "unexpected adapter failure"
        );
    }
}
