use crate::{
    api::{
        auth::SecureCookieStorage,
        oauth::{
            github::{GithubOauthService, GITHUB_ACCEPT_TYPE, GITHUB_EMAILS_URL, GITHUB_USER_URL}, Error,
            CSRF_COOKIE_NAME,
        },
        USER_AGENT_VALUE,
    },
    models::{ApiUser, ApiUserRepository, NewApiUser},
};
use axum::extract::State;
/// This module contains the request handlers for the GitHub OAuth flow.
/// It includes functions for login, logout, and authorization.
/// These handlers are used by the Axum framework to handle incoming HTTP requests.
use axum::{
    extract::Query,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use cookie::SameSite;
use http::{
    header::{ACCEPT, USER_AGENT},
    HeaderValue,
};
use oauth2::{
    AccessToken, AuthorizationCode, CsrfToken, ErrorResponse, RevocableToken, Scope,
    TokenIntrospectionResponse, TokenResponse,
};
use reqwest::redirect::Policy;
use serde::Deserialize;
use std::{fmt::Debug, time::Duration};
use tracing::{debug, error, trace};
use crate::api::auth::login;

/// Handles the login request.
/// Generates the authorization URL and CSRF token, sets the CSRF token as a cookie,
/// and redirects the user to the authorization URL.
///
/// # Parameters
///
/// - `oauth_client`: The `GitHubOAuthClient` instance.
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
pub(super) async fn oauth_login(
    State(oauth_client): State<GithubOauthService>,
    cookie_storage: SecureCookieStorage,
) -> Result<impl IntoResponse, Error> {
    // Generate the authorization URL and CSRF token
    let (auth_url, csrf_token) = oauth_client
        .oauth_client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("read:user".to_string()))
        .add_scope(Scope::new("user:email".to_string()))
        .url();

    // Serialize the CSRF token as a string
    let csrf_cookie_value = serde_json::to_string(&csrf_token)?;

    // Create a new CSRF token cookie
    let mut csrf_cookie = Cookie::new(CSRF_COOKIE_NAME, csrf_cookie_value);

    // Set cookie attributes
    csrf_cookie.set_http_only(true);
    csrf_cookie.set_secure(true);
    csrf_cookie.set_same_site(SameSite::Lax);
    csrf_cookie.set_max_age(cookie::time::Duration::minutes(5));
    csrf_cookie.set_path("/");

    // Add the CSRF token cookie to the cookie jar
    let cookie_storage = cookie_storage.add(csrf_cookie);

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
pub(super) async fn authorize(
    State(service): State<GithubOauthService>,
    Query(query): Query<AuthRequest>,
    cookie_storage: SecureCookieStorage,
) -> Result<Response, Error> {
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|e| Error::FetchUser(e.to_string()))?;
    
    // Exchange the authorization code for an access token
    let token = service
        .oauth_client
        .exchange_code(AuthorizationCode::new(query.code.clone()))
        .request_async(&client)
        .await
        .map_err(|e| {
            error!("OAuth flow with GitHub failed. Cannot exchange authorization code: {e:?}");
            Error::OauthToken(e.to_string())
        })?;

    // Get the CSRF token cookie from the cookie jar
    let mut csrf_cookie = cookie_storage
        .get(CSRF_COOKIE_NAME)
        .ok_or(Error::MissingCSRFCookie)?
        .clone();

    // Set cookie attributes
    csrf_cookie.set_same_site(SameSite::Lax);
    csrf_cookie.set_http_only(true);
    csrf_cookie.set_secure(true);
    csrf_cookie.set_path("/");

    // Deserialize the CSRF token from the cookie value
    let csrf_token: CsrfToken = serde_json::from_str(csrf_cookie.value())?;

    // Validate the CSRF token
    if query.state != *csrf_token.secret() {
        return Err(Error::CSRFTokenMismatch);
    }
    
    let api_user = service.fetch_user(token.access_token()).await?;

    let cookie_storage = login(api_user, cookie_storage)?;

    // Remove the CSRF token cookie
    let cookie_storage = cookie_storage.remove(csrf_cookie);

    // Return the updated cookie jar and a redirect response to the home page
    Ok((cookie_storage, Redirect::to("/")).into_response())
}
