//! ALTCHA challenge endpoint and verification helpers for public forms.

use axum::{
    extract::Extension,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use sigma_human_check::{HumanCheck, HumanCheckError};

/// `GET /human-check/challenge` — fresh signed PoW challenge (JSON).
pub async fn challenge(Extension(check): Extension<HumanCheck>) -> Response {
    if !check.is_enabled() {
        return (StatusCode::NOT_FOUND, "human check disabled").into_response();
    }
    match check.issue_challenge() {
        Ok(challenge) => Json(challenge).into_response(),
        Err(err) => {
            tracing::error!(?err, "failed to issue human-check challenge");
            (StatusCode::INTERNAL_SERVER_ERROR, "challenge unavailable").into_response()
        }
    }
}

/// User-facing message when verification fails on form submit.
#[must_use]
pub fn rejection_message(error: &HumanCheckError) -> String {
    match error {
        HumanCheckError::Missing => {
            "Please wait for verification to finish, then try again.".into()
        }
        HumanCheckError::Rejected | HumanCheckError::Altcha(_) | HumanCheckError::Json(_) => {
            "Human verification failed. Please try again.".into()
        }
        HumanCheckError::Config(_) => "Human verification is temporarily unavailable.".into(),
    }
}

/// Verify the `altcha` form field; returns `Ok(())` when disabled or valid.
pub fn verify_field(check: &HumanCheck, payload: &str) -> Result<(), HumanCheckError> {
    check.verify_payload_or_skip(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejection_messages_are_user_safe() {
        assert!(rejection_message(&HumanCheckError::Missing).contains("wait"));
        assert!(rejection_message(&HumanCheckError::Rejected).contains("failed"));
    }
}
