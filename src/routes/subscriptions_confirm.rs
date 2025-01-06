//! src/routes/subscriptions_confirm.rs

use actix_web::{http::StatusCode, web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{domain::SubscriptionToken, error::error_chain_fmt};

#[derive(thiserror::Error)]
pub enum ConfirmationError {
    #[error("{0}")]
    ValidationError(String),
    #[error("{0}")]
    AuthorizationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for ConfirmationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for ConfirmationError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            ConfirmationError::ValidationError(_) => StatusCode::BAD_REQUEST,
            ConfirmationError::AuthorizationError(_) => StatusCode::UNAUTHORIZED,
            ConfirmationError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

impl TryFrom<Parameters> for SubscriptionToken {
    type Error = String;

    fn try_from(value: Parameters) -> Result<Self, Self::Error> {
        SubscriptionToken::parse(value.subscription_token)
    }
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, pool))]
pub async fn confirm(
    parameters: web::Query<Parameters>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, ConfirmationError> {
    let subscription_token = parameters
        .0
        .try_into()
        .map_err(ConfirmationError::ValidationError)?;

    let id = get_subscriber_id_from_token(&pool, &subscription_token)
        .await
        .context("Failed to get subscriber ID from token if it exists")?;
    match id {
        None => Err(ConfirmationError::AuthorizationError(
            "Subscription token not found.".to_string(),
        )),
        Some(subscriber_id) => {
            confirm_subscriber(&pool, subscriber_id)
                .await
                .context("Failed to confirm subscriber")?;
            Ok(HttpResponse::Ok().finish())
        }
    }
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(pool, subscriber_id))]
pub async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(pool, subscription_token))]
pub async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &SubscriptionToken,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens \
        WHERE subscription_token = $1",
        subscription_token.as_ref(),
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.map(|r| r.subscriber_id))
}
