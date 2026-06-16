//! Integration tests for [`liter_llm_proxy::secrets::AwsSecretsManagerProvider`].
//!
//! The original brief asks for a test that mocks AWS Secrets Manager and
//! asserts that a `ResourceNotFoundException` SDK error is translated to
//! [`SecretError::NotFound`].
//!
//! `aws-smithy-mocks-experimental` (and the `aws-sdk-secretsmanager` `test`
//! feature) are not in `dev-dependencies` for this crate (see
//! `crates/liter-llm-proxy/Cargo.toml`).  The test below is therefore gated
//! behind `#[ignore]` so the test file compiles and is discoverable by
//! `cargo test`, while making the missing harness explicit.

#![cfg(feature = "secrets-aws")]

/// `AwsSecretsManagerProvider::get` must translate
/// `ResourceNotFoundException` into [`SecretError::NotFound`].
///
/// To enable:
///   1. Add `aws-smithy-mocks-experimental` (or enable the `aws-sdk-secretsmanager`
///      `test` feature) in `crates/liter-llm-proxy/Cargo.toml` dev-deps.
///   2. Remove the `#[ignore]` attribute below.
#[tokio::test]
#[ignore = "aws-smithy-mocks-experimental not in dev-dependencies; enable once available"]
async fn fetch_returns_error_on_resource_not_found() {
    // Pseudo-code body — fully realisable once a mocking harness is wired in:
    //
    // use aws_sdk_secretsmanager::operation::get_secret_value::GetSecretValueError;
    // use aws_smithy_mocks_experimental::{mock, mock_client, RuleMode};
    //
    // let not_found = mock!(aws_sdk_secretsmanager::Client::get_secret_value)
    //     .then_error(|| GetSecretValueError::ResourceNotFoundException(/* … */));
    // let client = mock_client!(aws_sdk_secretsmanager, RuleMode::Sequential, &[&not_found]);
    //
    // let provider = AwsSecretsManagerProvider::from_client(client, std::time::Duration::from_secs(60));
    // let err = provider.get("missing").await.expect_err("404 must error");
    // assert!(matches!(err, SecretError::NotFound(_)));
}
