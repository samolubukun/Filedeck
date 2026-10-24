//! Integration tests for FileDeck's error handling, HTTP status mapping, and JSON responses.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use filedeck_backend::error::AppError;

#[test]
fn test_app_error_not_found_status_and_body() {
    let err = AppError::NotFound("Document with id 999 was not found in the archive".to_string());
    assert_eq!(err.to_string(), "Not found: Document with id 999 was not found in the archive");

    let resp = err.into_response();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[test]
fn test_app_error_validation_and_bad_request() {
    let val_err = AppError::Validation("Archive serial number must be greater than zero".to_string());
    assert_eq!(val_err.to_string(), "Validation error: Archive serial number must be greater than zero");
    let resp1 = val_err.into_response();
    assert_eq!(resp1.status(), StatusCode::BAD_REQUEST);

    let bad_req = AppError::BadRequest("Invalid query parameters provided".to_string());
    assert_eq!(bad_req.to_string(), "Bad request: Invalid query parameters provided");
    let resp2 = bad_req.into_response();
    assert_eq!(resp2.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_app_error_conflict() {
    let conflict = AppError::Conflict("A document with this SHA256 checksum already exists".to_string());
    assert_eq!(conflict.to_string(), "Conflict: A document with this SHA256 checksum already exists");

    let resp = conflict.into_response();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[test]
fn test_app_error_internal() {
    let internal = AppError::Internal("Worker service communication failure".to_string());
    assert_eq!(internal.to_string(), "Internal server error: Worker service communication failure");

    let resp = internal.into_response();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_app_error_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Access denied to media directory");
    let app_err: AppError = io_err.into();

    assert!(app_err.to_string().contains("Access denied"));
    let resp = app_err.into_response();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
