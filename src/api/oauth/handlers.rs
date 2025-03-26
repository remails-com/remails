use crate::{
    api::{
        USER_AGENT_VALUE,
        auth::{CookieStorage, SESSION_COOKIE_NAME},
        oauth::{
            CSRF_COOKIE_NAME, Error,
            github::{GITHUB_ACCEPT_TYPE, GITHUB_EMAILS_URL, GITHUB_USER_URL, GithubOauthService},
        },
    },
    models::{ApiUser, ApiUserRepository, NewApiUser, UserCookie},
};
use axum::extract::State;
/// This module contains the request handlers for the GitHub OAuth flow.
/// It includes functions for login, logout, and authorization.
/// These handlers are used by the Axum framework to handle incoming HTTP requests.
use axum::{
    extract::Query,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use cookie::SameSite;
use http::{
    HeaderValue,
    header::{ACCEPT, USER_AGENT},
};
use oauth2::{AccessToken, AuthorizationCode, CsrfToken, Scope, TokenResponse};
use reqwest::redirect::Policy;
use serde::Deserialize;
use std::{fmt::Debug, time::Duration};
use tracing::{debug, error, trace};

/// Handles the login request.
/// Generates the authorization URL and CSRF token, sets the CSRF token as a cookie,
/// and redirects the user to the authorization URL.
///
/// # Parameters
///
/// - `service`: The `GithubOauthService` instance.
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
pub(super) async fn login(
    service: GithubOauthService,
    jar: CookieJar,
) -> Result<impl IntoResponse, Error> {
    // Generate the authorization URL and CSRF token
    let (auth_url, csrf_token) = service
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
    let updated_jar = jar.add(csrf_cookie);

    // Return the updated cookie jar and a redirect response to the authorization URL
    Ok((updated_jar, Redirect::to(auth_url.to_string().as_str())))
}

/// Represents the request parameters for the authorization request.
#[derive(Debug, Deserialize)]
pub(super) struct AuthRequest {
    code: String,
    state: String,
}

#[derive(Debug, Deserialize)]
struct GitHubUser {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
struct GitHubEmail {
    email: String,
    verified: bool,
    primary: bool,
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
    service: GithubOauthService,
    Query(query): Query<AuthRequest>,
    cookie_storage: CookieStorage,
    jar: CookieJar,
    State(user_repo): State<ApiUserRepository>,
) -> Result<Response, Error> {
    let private_jar = cookie_storage.jar;

    // Create a new HTTP client
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
    let mut csrf_cookie = jar
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

    // Fetch user data from the GitHub API
    let user_data: GitHubUser = client
        .get(GITHUB_USER_URL)
        .header(ACCEPT, HeaderValue::from_static(GITHUB_ACCEPT_TYPE))
        .header(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE))
        .bearer_auth(token.access_token().secret())
        .send()
        .await
        .map_err(|e| Error::FetchUser(e.to_string()))?
        .json()
        .await
        .map_err(|e| Error::ParseUser(e.to_string()))?;

    let api_user = if let Some(existing_user) = user_repo.find_by_github_id(user_data.id).await? {
        trace!(
            user_id = existing_user.id().to_string(),
            "Signed in with GitHub for existing user"
        );
        existing_user
    } else {
        let new = sign_up(user_repo, user_data.id, token.access_token(), &client).await?;
        debug!(
            user_id = new.id().to_string(),
            "Signed up new user via GitHub"
        );
        new
    };

    // Serialize the user data as a string
    let cookie: UserCookie = api_user.into();
    let session_cookie_value = serde_json::to_string(&cookie)?;

    // Create a new session cookie
    let mut session_cookie = Cookie::new(SESSION_COOKIE_NAME, session_cookie_value);
    session_cookie.set_http_only(true);
    session_cookie.set_secure(true);
    session_cookie.set_same_site(SameSite::Lax);
    session_cookie.set_max_age(cookie::time::Duration::days(7));
    session_cookie.set_path("/");

    // Remove the CSRF token cookie and add the session cookie to the cookie jar
    let updated_jar = jar.remove(csrf_cookie);
    let updated_private_jar = private_jar.add(session_cookie);

    // Return the updated cookie jar and a redirect response to the home page
    Ok((updated_jar, updated_private_jar, Redirect::to("/")).into_response())
}

async fn sign_up(
    user_repo: ApiUserRepository,
    github_user_id: i64,
    github_token: &AccessToken,
    http_client: &reqwest::Client,
) -> Result<ApiUser, Error> {
    // Fetch email addresses from the GitHub API
    let emails: Vec<GitHubEmail> = http_client
        .get(GITHUB_EMAILS_URL)
        .header(ACCEPT, HeaderValue::from_static(GITHUB_ACCEPT_TYPE))
        .header(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE))
        .bearer_auth(github_token.secret())
        .send()
        .await
        .map_err(|e| Error::FetchUser(e.to_string()))?
        .json()
        .await
        .map_err(|e| Error::ParseUser(e.to_string()))?;

    let mut email = None;

    // find the email address that is allowed
    for e in &emails {
        if e.verified && e.primary {
            email = Some(e.email.clone());
        }
    }

    // if no primary email address is found, return an error
    let email = match email {
        Some(email) => email
            .parse()
            .map_err(|e| Error::Other(format!("Invalid email address in GitHub: {e}")))?,
        None => Err(Error::Other(
            "No verified and primary email address found".to_string(),
        ))?,
    };

    Ok(user_repo
        .create(&NewApiUser {
            email,
            roles: vec![],
            github_user_id: Some(github_user_id),
        })
        .await?)
}
