//! src/routes/admin/password/post.rs

use crate::authentication::{validate_credentials, AuthError, Credentials, UserId};
use crate::routes::admin::dashboard::get_username;
use crate::utils::{e500, see_other};
use actix_web::{web, HttpResponse};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use actix_web_flash_messages::FlashMessage;

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

pub async fn change_password(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match.",
        )
        .send();
        return Ok(see_other("/admin/password"));
    }

    let new_password_length = form.new_password.expose_secret().len();
    if !(12..=129).contains(&new_password_length) {
        FlashMessage::error(
            "Invalid password length - the new password must be between 12 and 129 characters.",
        )
        .send();
        return Ok(see_other("/admin/password"));
    }

    let user_id = user_id.into_inner();
    let username = get_username(*user_id, &pool).await.map_err(e500)?;

    let credentials = Credentials {
        username,
        password: form.0.current_password,
    };
    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect.").send();
                Ok(see_other("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(e500(e)),
        };
    }
    crate::authentication::change_password(*user_id, form.0.new_password, &pool)
        .await
        .map_err(e500)?;
    FlashMessage::error("Your password has been changed.").send();
    Ok(see_other("/admin/password"))
}
