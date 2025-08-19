use crate::{
    api::{
        ApiState,
        auth::{SecureCookieStorage, login},
        oauth::{CSRF_COOKIE_NAME, Error, OAuthService},
    },
    models::ApiUser,
};
use axum::extract::{FromRef, State};
/// This module contains the request handlers for the OAuth flow.
/// It includes functions for login, and authorization.
/// These handlers are used by the Axum framework to handle incoming HTTP requests.
use axum::{
    extract::Query,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::Cookie;
use cookie::SameSite;
use oauth2::{AuthorizationCode, CsrfToken, TokenResponse};
use reqwest::redirect::Policy;
use serde::Deserialize;
use std::{fmt::Debug, time::Duration};
use tracing::error;

static REDIRECT_COOKIE_NAME: &str = "redirect";

#[derive(Debug, Deserialize)]
pub(super) struct RedirectParam {
    redirect: Option<String>,
}

fn set_cookie_attributes(cookie: &mut Cookie) {
    cookie.set_http_only(true);
    cookie.set_secure(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_path("/");
}

fn new_cookie<'a>(name: &'a str, value: String) -> Cookie<'a> {
    let mut cookie = Cookie::new(name, value);

    // Set cookie attributes
    set_cookie_attributes(&mut cookie);
    cookie.set_max_age(cookie::time::Duration::minutes(5));

    cookie
}

/// Handles the login request.
/// Generates the authorization URL and CSRF token, sets the CSRF token as a cookie,
/// and redirects the user to the authorization URL.
///
/// # Parameters
///
/// - `oauth_service`: The `OAuthService` instance.
/// - `jar`: The private cookie jar to store the CSRF token cookie.
///
/// # Returns
///
/// Returns a `Result` containing the updated cookie jar and a `Redirect` response.
/// If successful, the user will be redirected to the authorization URL.
///
/// # Errors
///
/// Returns an `Error` if there is an issue generating the CSRF token or setting the cookie.
pub(super) async fn oauth_login<T>(
    State(oauth_service): State<T>,
    Query(redirect): Query<RedirectParam>,
    cookie_storage: SecureCookieStorage,
) -> Result<impl IntoResponse, Error>
where
    T: OAuthService,
    T: FromRef<ApiState>,
{
    // Generate the authorization URL and CSRF token
    let mut oauth = oauth_service
        .oauth_client()
        .authorize_url(CsrfToken::new_random);
    for scope in T::scopes() {
        oauth = oauth.add_scope(scope);
    }
    let (auth_url, csrf_token) = oauth.url();

    // Serialize the CSRF token as a string
    let csrf_cookie_value = serde_json::to_string(&csrf_token)?;

    // Create a new CSRF token cookie and add it to the cookie jar
    let mut cookie_storage = cookie_storage.add(new_cookie(CSRF_COOKIE_NAME, csrf_cookie_value));

    // Store redirect path in temporary cookie
    if let Some(redirect) = redirect.redirect {
        cookie_storage = cookie_storage.add(new_cookie(REDIRECT_COOKIE_NAME, redirect));
    }

    // Return the updated cookie jar and a redirect response to the authorization URL
    Ok((cookie_storage, Redirect::to(auth_url.to_string().as_str())))
}

/// Represents the request parameters for the authorization request.
#[derive(Debug, Deserialize)]
pub(super) struct AuthRequest {
    code: String,
    state: String,
}

/// Handles the authorization request.
/// Exchanges the authorization code for an access token,
/// validates the CSRF token, fetches user data and organizations,
/// and sets the session cookie if the user is authorized.
///
/// # Parameters
///
/// - `service`: The `GithubOauthService` instance.
/// - `Query(query)`: The query parameters containing the authorization code and CSRF token.
/// - `jar`: The private cookie jar containing the CSRF token cookie.
///
/// # Returns
///
/// Returns a `Result` containing the updated cookie jar and a redirect response to the home page.
/// If successful, the user will be redirected to the home page with the session cookie set.
///
/// # Errors
///
/// Returns an `Error` if there is an issue exchanging the authorization code for an access token,
/// validating the CSRF token, fetching user data or organizations, or setting the session cookie.
pub(super) async fn authorize<T>(
    State(service): State<T>,
    Query(query): Query<AuthRequest>,
    mut cookie_storage: SecureCookieStorage,
    logged_in_api_user: Option<ApiUser>,
) -> Result<Response, Error>
where
    T: OAuthService,
    T: FromRef<ApiState>,
{
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|e| Error::FetchUser(e.to_string()))?;

    // Exchange the authorization code for an access token
    let token = service
        .oauth_client()
        .exchange_code(AuthorizationCode::new(query.code.clone()))
        .request_async(&client)
        .await
        .map_err(|e| {
            error!("OAuth flow failed. Cannot exchange authorization code: {e:?}");
            Error::OauthToken(e.to_string())
        })?;

    // Get the CSRF token cookie from the cookie jar
    let mut csrf_cookie = cookie_storage
        .get(CSRF_COOKIE_NAME)
        .ok_or(Error::MissingCSRFCookie)?
        .clone();
    set_cookie_attributes(&mut csrf_cookie);

    // Deserialize the CSRF token from the cookie value
    let csrf_token: CsrfToken = serde_json::from_str(csrf_cookie.value())?;

    // Validate the CSRF token
    if query.state != *csrf_token.secret() {
        return Err(Error::CSRFTokenMismatch);
    }

    let api_user = service
        .fetch_user(token.access_token(), logged_in_api_user)
        .await?;

    cookie_storage = login(&api_user, cookie_storage)?;

    // Remove the CSRF token cookie
    cookie_storage = cookie_storage.remove(csrf_cookie);

    // Return the updated cookie jar and a redirect response to the home page
    if let Some(mut redirect_cookie) = cookie_storage.get(REDIRECT_COOKIE_NAME) {
        set_cookie_attributes(&mut redirect_cookie);
        let redirect = redirect_cookie.value().to_owned();
        cookie_storage = cookie_storage.remove(redirect_cookie);
        tracing::info!("Redirecting to {}", redirect);
        Ok((cookie_storage, Redirect::to(&redirect)).into_response())
    } else {
        Ok((cookie_storage, Redirect::to("/")).into_response())
    }
}
