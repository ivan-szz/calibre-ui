//! The Kobo store API under `/kobo/{token}`, for Kobo e-readers. The token in the
//! URL identifies the user, since devices send no session; see `features::kobo`.

use serde_json::{Value, json};
use topcoat::{
    Result,
    context::Cx,
    router::{
        StatusCode,
        content::Json,
        error::RouterErrorExt,
        header, module_param, path_param, request, route,
    },
};

use crate::features::{
    auth::{Perm, User},
    kobo::{self, KoboState},
    library::{self, Book},
};

module_param!(pub kobo_token);
topcoat::router::path_param!(kobo_uuid);
topcoat::router::path_param!(kobo_book: i64, error = not_found);
topcoat::router::path_param!(kobo_format);
topcoat::router::path_param!(*kobo_rest);

/// Books per sync response; the device asks again while `x-kobo-sync` says `continue`.
const SYNC_BATCH: usize = 100;

/// The user the URL's token belongs to, if they may download books.
async fn device_user(cx: &Cx) -> Result<User> {
    let user = User::by_kobo_token(cx, path_param::<KoboToken>(cx)).await?.ok_or_unauthorized()?;
    Ok(Some(user).filter(|u| u.can(Perm::Download)).ok_or_forbidden()?)
}

/// This sync endpoint as the device reaches it, `…/kobo/{token}`.
fn base_url(cx: &Cx) -> String {
    let origin = std::env::var("PUBLIC_URL").ok().filter(|u| !u.is_empty()).unwrap_or_else(|| {
        let host = request::headers(cx).get("host").and_then(|h| h.to_str().ok()).unwrap_or("localhost");
        format!("http://{host}")
    });
    format!("{}/kobo/{}", origin.trim_end_matches('/'), path_param::<KoboToken>(cx))
}

async fn book_by_uuid(cx: &Cx, user: &User) -> Result<Book> {
    let hidden = user.hidden();
    let id = library::book_id_by_uuid(path_param::<KoboUuid>(cx), &hidden)?.ok_or_not_found()?;
    Ok(library::book(id, &hidden)?.ok_or_not_found()?)
}

/// The resources the device uses to find the store API. Library, metadata, and covers
/// point here. With the store proxy on, the rest come from Kobo's own answer, as in
/// calibre-web; otherwise from a built-in list of Kobo's addresses.
#[route(GET "./v1/initialization")]
pub async fn initialization(cx: &Cx) -> Result<([(header::HeaderName, &'static str); 1], Json<Value>)> {
    device_user(cx).await?;
    let base = base_url(cx);
    let store = store();
    let mut resources = json!({
        "account_page": "https://www.kobo.com/account/settings",
        "account_page_rakuten": "https://my.rakuten.co.jp/",
        "book_detail_page": "https://store.kobo.com/{culture}/ebook/{slug}",
        "categories": format!("{store}/v1/categories"),
        "content_access_book": format!("{store}/v1/products/books/{{ProductId}}/access"),
        "customer_care_live_chat": "https://v2.zopim.com/widget/livechat.html?key=Y6gwUmnu4OATxN3Tli4Av9bYN319BTdO",
        "deals": format!("{store}/v1/deals"),
        "device_auth": format!("{base}/v1/auth/device"),
        "device_refresh": format!("{base}/v1/auth/refresh"),
        "get_tests_request": format!("{store}/v1/analytics/gettests"),
        "image_host": base.clone(),
        "image_url_quality_template": format!("{base}/images/{{ImageId}}/{{Width}}/{{Height}}/{{Quality}}/{{IsGreyscale}}/image.jpg"),
        "image_url_template": format!("{base}/images/{{ImageId}}/{{Width}}/{{Height}}/false/image.jpg"),
        "library_book": format!("{base}/v1/user/library/books/{{LibraryItemId}}"),
        "library_items": format!("{base}/v1/user/library"),
        "library_metadata": format!("{base}/v1/library/{{Ids}}/metadata"),
        "library_sync": format!("{base}/v1/library/sync"),
        "post_analytics_event": format!("{store}/v1/analytics/event"),
        "reading_state": format!("{base}/v1/library/{{Ids}}/state"),
        "store_home": "www.kobo.com/{region}/{language}",
        "user_profile": format!("{base}/v1/user/profile"),
    });
    if proxy_enabled() {
        // Kobo's full list, with this app's own entries laid over it.
        let path = format!("{store}/v1/initialization");
        match forward(cx, "GET", &path, Vec::new()).await {
            Ok((status, _, body)) if status.is_success() => {
                if let Some(Value::Object(store_resources)) =
                    serde_json::from_slice::<Value>(&body).ok().and_then(|v| v.get("Resources").cloned())
                {
                    let mut merged = store_resources;
                    for key in ["image_host", "image_url_quality_template", "image_url_template", "library_sync", "library_metadata", "reading_state", "device_auth", "device_refresh"] {
                        merged.insert(key.to_owned(), resources[key].clone());
                    }
                    resources = Value::Object(merged);
                }
            }
            Ok((status, ..)) => eprintln!("Kobo store initialization failed: {status}"),
            Err(error) => eprintln!("Kobo store initialization failed: {error:#}"),
        }
    }
    Ok(([(header::HeaderName::from_static("x-kobo-apitoken"), "e30=")], Json(json!({ "Resources": resources }))))
}

/// Kobo's store API, which the proxy forwards to. `KOBO_STORE_URL` replaces it, for
/// testing.
fn store() -> String {
    std::env::var("KOBO_STORE_URL").ok().filter(|u| !u.is_empty()).unwrap_or_else(|| "https://storeapi.kobo.com".to_owned())
}

/// Whether requests this app does not answer go on to the Kobo store, as calibre-web's
/// "proxy unknown requests to Kobo Store" option does. Set `KOBO_STORE_PROXY=1`.
fn proxy_enabled() -> bool {
    std::env::var("KOBO_STORE_PROXY").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// Request headers worth passing to the store: the device's store credentials and
/// what describes the body. Cookies and host stay behind.
fn forwarded_header(name: &str) -> bool {
    name == "authorization" || name == "content-type" || name == "accept" || name == "user-agent" || name.starts_with("x-kobo-")
}

/// Sends the request on to `url` and returns the store's status, content type, and body.
async fn forward(cx: &Cx, method: &str, url: &str, body: Vec<u8>) -> Result<(StatusCode, String, Vec<u8>)> {
    let headers: Vec<(String, String)> = request::headers(cx)
        .iter()
        .filter(|(name, _)| forwarded_header(name.as_str()))
        .filter_map(|(name, value)| Some((name.as_str().to_owned(), value.to_str().ok()?.to_owned())))
        .collect();
    let (method, url) = (method.to_owned(), url.to_owned());
    tokio::task::spawn_blocking(move || -> Result<(StatusCode, String, Vec<u8>)> {
        let error = |e: &dyn std::fmt::Display| topcoat::Error::msg(format!("Kobo store: {e}"));
        let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).build().into();
        let mut builder = ureq::http::Request::builder().method(method.as_str()).uri(&url);
        for (name, value) in &headers {
            builder = builder.header(name, value);
        }
        let mut response = agent.run(builder.body(body).map_err(|e| error(&e))?).map_err(|e| error(&e))?;
        let status = StatusCode::from_u16(response.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/json")
            .to_owned();
        let bytes = response.body_mut().read_to_vec().map_err(|e| error(&e))?;
        Ok((status, content_type, bytes))
    })
    .await
    .map_err(|e| topcoat::Error::msg(e.to_string()))?
}

/// The books changed since the device's sync token, oldest change first, in batches.
#[route(GET "./v1/library/sync")]
pub async fn sync(cx: &Cx) -> Result<([(header::HeaderName, String); 2], Json<Value>)> {
    let user = device_user(cx).await?;
    let hidden = user.hidden();
    let since = kobo::read_sync_token(request::headers(cx).get("x-kobo-synctoken").and_then(|h| h.to_str().ok()));
    let changes = library::kobo_changes(&since, SYNC_BATCH + 1, &hidden)?;
    let more = changes.len() > SYNC_BATCH;
    let base = base_url(cx);
    let mut entitlements = Vec::new();
    let mut newest = since;
    for (id, last_modified) in changes.into_iter().take(SYNC_BATCH) {
        if let Some(book) = library::book(id, &hidden)? {
            let state = KoboState::get(cx, user.id, &book).await?;
            entitlements.push(kobo::new_entitlement(&book, &base, state));
        }
        newest = last_modified;
    }
    Ok((
        [
            (header::HeaderName::from_static("x-kobo-synctoken"), kobo::sync_token(&newest)),
            (header::HeaderName::from_static("x-kobo-sync"), if more { "continue".into() } else { String::new() }),
        ],
        Json(Value::Array(entitlements)),
    ))
}

#[route(GET "./v1/library/{kobo_uuid}/metadata")]
pub async fn metadata(cx: &Cx) -> Result<Json<Value>> {
    let user = device_user(cx).await?;
    let book = book_by_uuid(cx, &user).await?;
    Ok(Json(json!([kobo::metadata(&book, &base_url(cx))])))
}

#[route(GET "./v1/library/{kobo_uuid}/state")]
pub async fn get_state(cx: &Cx) -> Result<Json<Value>> {
    let user = device_user(cx).await?;
    let book = book_by_uuid(cx, &user).await?;
    Ok(Json(json!([KoboState::current(cx, user.id, &book).await?])))
}

/// Saves the reading position and status the device reports.
#[route(PUT "./v1/library/{kobo_uuid}/state")]
pub async fn put_state(cx: &Cx, Json(body): Json<Value>) -> Result<Json<Value>> {
    let user = device_user(cx).await?;
    let book = book_by_uuid(cx, &user).await?;
    let state = body
        .get("ReadingStates")
        .and_then(|s| s.get(0))
        .cloned()
        .filter(Value::is_object)
        .ok_or_bad_request("expected ReadingStates")?;
    let last_modified = state.get("LastModified").cloned().unwrap_or(Value::Null);
    KoboState::save(cx, user.id, &book, state).await?;
    Ok(Json(json!({
        "RequestResult": "Success",
        "UpdateResults": [{
            "EntitlementId": book.uuid,
            "CurrentBookmarkResult": { "Result": "Success" },
            "StatisticsResult": { "Result": "Success" },
            "StatusInfoResult": { "Result": "Success" },
            "LastModified": last_modified,
            "PriorityTimestamp": last_modified,
        }],
    })))
}

/// Removing a book on the device leaves it in the library.
#[route(DELETE "./v1/library/{kobo_uuid}")]
pub async fn remove(cx: &Cx) -> Result<StatusCode> {
    device_user(cx).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The book file, KEPUB or EPUB.
#[route(GET "./download/{kobo_book}/{kobo_format}")]
pub async fn download(cx: &Cx) -> Result<([(header::HeaderName, &'static str); 1], Vec<u8>)> {
    let user = device_user(cx).await?;
    let format = path_param::<KoboFormat>(cx);
    let (file, _) = library::format_file(*path_param::<KoboBook>(cx)?, format, &user.hidden())?.ok_or_not_found()?;
    let bytes = tokio::fs::read(file).await.ok().ok_or_not_found()?;
    Ok(([(header::CONTENT_TYPE, library::content_type(format))], bytes))
}

async fn cover_image(cx: &Cx) -> Result<([(header::HeaderName, &'static str); 1], Vec<u8>)> {
    let user = device_user(cx).await?;
    let hidden = user.hidden();
    let id = library::book_id_by_uuid(path_param::<KoboUuid>(cx), &hidden)?.ok_or_not_found()?;
    let file = library::cover_file(id, &hidden)?.ok_or_not_found()?;
    let bytes = tokio::fs::read(file).await.ok().ok_or_not_found()?;
    Ok(([(header::CONTENT_TYPE, "image/jpeg")], bytes))
}

/// The cover, by `image_url_template`. The size is the device's to scale to.
#[route(GET "./images/{kobo_uuid}/{kobo_w}/{kobo_h}/{kobo_grey}/image.jpg")]
pub async fn cover(cx: &Cx) -> Result<([(header::HeaderName, &'static str); 1], Vec<u8>)> {
    cover_image(cx).await
}

/// The cover, by `image_url_quality_template`.
#[route(GET "./images/{kobo_uuid}/{kobo_w}/{kobo_h}/{kobo_q}/{kobo_grey}/image.jpg")]
pub async fn cover_quality(cx: &Cx) -> Result<([(header::HeaderName, &'static str); 1], Vec<u8>)> {
    cover_image(cx).await
}

/// Signs the device in. The URL's token is the real credential, so these tokens only
/// satisfy the device, as in calibre-web.
#[route(POST "./v1/auth/device")]
pub async fn auth_device(cx: &Cx) -> Result<Json<Value>> {
    device_tokens(cx).await
}

#[route(POST "./v1/auth/refresh")]
pub async fn auth_refresh(cx: &Cx) -> Result<Json<Value>> {
    device_tokens(cx).await
}

async fn device_tokens(cx: &Cx) -> Result<Json<Value>> {
    let user = device_user(cx).await?;
    let token = path_param::<KoboToken>(cx).to_owned();
    Ok(Json(json!({
        "AccessToken": token,
        "RefreshToken": token,
        "TokenType": "Bearer",
        "TrackingId": format!("calibre-ui-{}", user.id),
        "UserKey": format!("calibre-ui-{}", user.id),
    })))
}

/// The device's profile, library and store calls this app has no data for. With the
/// store proxy on they go to Kobo's store; otherwise empty answers keep the device
/// working without it.
#[route(* "./{*kobo_rest}")]
pub async fn other(cx: &Cx, body: request::Bytes) -> Result<(StatusCode, [(header::HeaderName, String); 1], Vec<u8>)> {
    device_user(cx).await?;
    let rest: Vec<&str> = path_param::<KoboRest>(cx).collect();
    if proxy_enabled() {
        let query = request::uri(cx).query().map(|q| format!("?{q}")).unwrap_or_default();
        let url = format!("{}/{}{query}", store(), rest.join("/"));
        let (status, content_type, bytes) = forward(cx, request::method(cx).as_str(), &url, body.to_vec()).await?;
        return Ok((status, [(header::CONTENT_TYPE, content_type)], bytes));
    }
    let empty = if rest.last() == Some(&"library") || rest.ends_with(&["library", "tags"]) { "[]" } else { "{}" };
    Ok((StatusCode::OK, [(header::CONTENT_TYPE, "application/json".to_owned())], empty.as_bytes().to_vec()))
}

topcoat::router::path_param!(kobo_w);
topcoat::router::path_param!(kobo_h);
topcoat::router::path_param!(kobo_q);
topcoat::router::path_param!(kobo_grey);
