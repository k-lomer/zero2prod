//! src/routes/subscriptions.rs

use actix_web::{web, HttpResponse, ResponseError};
use chrono::Utc;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName, SubscriptionToken};
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;

#[derive(serde::Deserialize)]
#[allow(dead_code)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { email, name })
    }
}

pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while \
                trying to store a subscription token."
        )
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

impl ResponseError for StoreTokenError {}

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, actix_web::Error> {
    let new_subscriber = match form.0.try_into() {
        Ok(new_subscriber) => new_subscriber,
        Err(_) => return Ok(HttpResponse::BadRequest().finish()),
    };

    let mut subscriber_id = match get_subscriber_id_from_email(&pool, &new_subscriber).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => return Ok(HttpResponse::InternalServerError().finish()),
    };

    let mut subscription_token = match subscriber_id {
        Some(subscriber_id) => match get_subscription_token_from_id(&pool, &subscriber_id).await {
            Ok(subscription_token) => subscription_token,
            Err(_) => return Ok(HttpResponse::InternalServerError().finish()),
        },
        None => None,
    };

    if subscription_token.is_none() {
        let mut transaction = match pool.begin().await {
            Ok(transaction) => transaction,
            Err(_) => return Ok(HttpResponse::InternalServerError().finish()),
        };

        if subscriber_id.is_none() {
            let new_subscriber_id = match insert_subscriber(&new_subscriber, &mut transaction).await
            {
                Ok(new_subscriber_id) => new_subscriber_id,
                Err(_) => return Ok(HttpResponse::InternalServerError().finish()),
            };
            subscriber_id = Some(new_subscriber_id);
        }

        let new_subscription_token = SubscriptionToken::generate();
        if let Some(subscriber_id) = subscriber_id {
            store_token(&mut transaction, subscriber_id, &new_subscription_token).await?;
            subscription_token = Some(new_subscription_token);
        } else {
            return Ok(HttpResponse::InternalServerError().finish());
        }

        if transaction.commit().await.is_err() {
            return Ok(HttpResponse::InternalServerError().finish());
        }
    }

    if let Some(subscription_token) = subscription_token {
        if send_confirmation_email(
            &email_client,
            new_subscriber,
            &base_url.0,
            &subscription_token,
        )
        .await
        .is_err()
        {
            return Ok(HttpResponse::InternalServerError().finish());
        }
    } else {
        return Ok(HttpResponse::InternalServerError().finish());
    }

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(new_subscriber, transaction)
)]
pub async fn insert_subscriber(
    new_subscriber: &NewSubscriber,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    );
    transaction.execute(query).await.map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, transaction)
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &SubscriptionToken,
) -> Result<(), StoreTokenError> {
    let query = sqlx::query!(
        r#"
        INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)
        "#,
        subscription_token.as_ref(),
        subscriber_id
    );
    transaction.execute(query).await.map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        StoreTokenError(e)
    })?;
    Ok(())
}

#[tracing::instrument(
    name = "Sending a confirmation email to a new subscriber",
    skip(email_client, new_subscriber, base_url, subscription_token)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &SubscriptionToken,
) -> Result<(), String> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url,
        subscription_token.as_ref(),
    );
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    let html_body = format!(
        "Welcome to our newsletter!<br />\
                    Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
}

#[tracing::instrument(name = "Get subscription token from id", skip(pool, subscriber_id))]
pub async fn get_subscription_token_from_id(
    pool: &PgPool,
    subscriber_id: &Uuid,
) -> Result<Option<SubscriptionToken>, String> {
    let result = sqlx::query!(
        "SELECT subscription_token FROM subscription_tokens \
        WHERE subscriber_id = $1",
        subscriber_id,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e.to_string()
    })?;
    result.map_or(Ok(None), |record| {
        SubscriptionToken::parse(record.subscription_token).map(Some)
    })
}

#[tracing::instrument(name = "Get subscriber id from email", skip(pool, subscriber))]
pub async fn get_subscriber_id_from_email(
    pool: &PgPool,
    subscriber: &NewSubscriber,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT id FROM subscriptions \
        WHERE email = $1",
        subscriber.email.as_ref(),
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(result.map(|r| r.id))
}
