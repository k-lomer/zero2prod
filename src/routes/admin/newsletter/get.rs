//! src/routes/admin/newsletter/get.rs

use actix_web::http::header::ContentType;
use actix_web::HttpResponse;

pub async fn newsletter_form() -> Result<HttpResponse, actix_web::Error> {
    Ok(HttpResponse::Ok().content_type(ContentType::html()).body(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Send a newsletter</title>
</head>
<body>
    <form action="/admin/newsletters" method="post">
        <label>Title
            <input
                placeholder="Enter newsletter title"
                name="title"
            >
        </label>
        <br>
<p><label for="text_content">Text Content:</label></p>
  <textarea id="text_content" name="text_content" rows="4" cols="50"></textarea>
        <br>
<p><label for="html_content">HTML Content:</label></p>
  <textarea id="html_content" name="html_content" rows="4" cols="50"></textarea>
        <br>
        <button type="submit">Send newsletter</button>
    </form>
    <p><a href="/admin/dashboard">&lt;- Back</a></p>
</body>
</html>"#
            .to_string(),
    ))
}
