use super::handlers::{authorize, login, logout};
use crate::api::{
    error::ApiError,
    oauth::{COOKIE_NAME, Error},
};
use axum::{
    RequestPartsExt, Router,
    extract::{FromRef, FromRequestParts, OptionalFromRequestParts},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use axum_extra::extract::{PrivateCookieJar, cookie};
use base64::prelude::*;
use http::{HeaderMap, request::Parts};
use oauth2::{
    AuthUrl, Client, ClientId, ClientSecret, EndpointNotSet, EndpointSet, RedirectUrl,
    StandardRevocableToken, TokenUrl,
    basic::{
        BasicClient, BasicErrorResponse, BasicRevocationErrorResponse,
        BasicTokenIntrospectionResponse, BasicTokenResponse,
    },
};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, env, fmt::Debug};
use tracing::warn;
use url::Url;

pub(super) static GITHUB_AUTH_URL: &str = "https://github.com/login/oauth/authorize";
pub(super) static GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
pub(super) static GITHUB_USER_URL: &str = "https://api.github.com/user";
pub(super) static GITHUB_EMAILS_URL: &str = "https://api.github.com/user/emails";
pub(super) static GITHUB_ACCEPT_TYPE: &str = "application/vnd.github+json";

type GitHubOAuthClient = Client<
    BasicErrorResponse,
    BasicTokenResponse,
    BasicTokenIntrospectionResponse,
    StandardRevocableToken,
    BasicRevocationErrorResponse,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet,
>;

/// Represents the GitHub OAuth service.
#[derive(Clone)]
pub struct GithubOauthService {
    pub(super) oauth_client: GitHubOAuthClient,
    pub(super) config: Config,
}

impl<S> FromRequestParts<S> for GithubOauthService
where
    GithubOauthService: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(GithubOauthService::from_ref(state))
    }
}

pub(crate) struct CookieStorage {
    pub(crate) jar: PrivateCookieJar,
}

impl<S> FromRequestParts<S> for CookieStorage
where
    GithubOauthService: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let service = GithubOauthService::from_ref(state);
        let jar =
            PrivateCookieJar::from_headers(&parts.headers, service.config.session_key.clone());

        Ok(Self { jar })
    }
}

/// Represents a user retrieved from GitHub.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: usize,
    pub login: String,
    pub email: String,
    pub avatar_url: String,
}

/// Represents the configuration for the GitHub OAuth service.
#[derive(Clone)]
pub struct Config {
    // github endpoints
    pub auth_url: Url,
    pub token_url: Url,
    // application specific settings / secrets
    pub session_key: cookie::Key,
    pub redirect_url: Url,
}

impl Default for Config {
    fn default() -> Self {
        let redirect_url = env::var("REDIRECT_URL")
            .expect("missing REDIRECT_URL from environment")
            .parse()
            .expect("failed to parse REDIRECT_URL");

        let session_key = match env::var("SESSION_KEY") {
            Ok(session_key_base64) => {
                let key_bytes = BASE64_STANDARD
                    .decode(session_key_base64)
                    .expect("SESSION_KEY env var must be valid base 64");
                cookie::Key::from(&key_bytes)
            }
            Err(_) => {
                warn!("Could not find SESSION_KEY; generating one");
                cookie::Key::generate()
            }
        };

        Self {
            auth_url: Url::parse(GITHUB_AUTH_URL).unwrap(),
            token_url: Url::parse(GITHUB_TOKEN_URL).unwrap(),
            session_key,
            redirect_url,
        }
    }
}

impl GithubOauthService {
    /// Creates a new instance of `GithubOauthService`.
    ///
    /// # Arguments
    ///
    /// * `config` - Optional configuration for the service. If not provided, default configuration will be used.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the `GithubOauthService` instance or an `Error` if there was an error creating the service.
    pub fn new(config: Option<Config>) -> Result<Self, Error> {
        let config = config.unwrap_or_default();
        let client_id = env::var("OAUTH_CLIENT_ID")
            .map_err(|_| Error::MissingEnvironmentVariable("OAUTH_CLIENT_ID"))?;
        let client_secret = env::var("OAUTH_CLIENT_SECRET")
            .map_err(|_| Error::MissingEnvironmentVariable("OAUTH_CLIENT_SECRET"))?;

        let oauth_client: Client<
            BasicErrorResponse,
            BasicTokenResponse,
            BasicTokenIntrospectionResponse,
            StandardRevocableToken,
            BasicRevocationErrorResponse,
            EndpointSet,
            EndpointNotSet,
            EndpointNotSet,
            EndpointNotSet,
            EndpointSet,
        > = BasicClient::new(ClientId::new(client_id))
            .set_client_secret(ClientSecret::new(client_secret))
            .set_auth_uri(AuthUrl::from_url(config.auth_url.clone()))
            .set_token_uri(TokenUrl::from_url(config.token_url.clone()))
            .set_redirect_uri(RedirectUrl::from_url(config.redirect_url.clone()));

        Ok(Self {
            oauth_client,
            config,
        })
    }

    /// Creates a router for the GitHub OAuth service.
    ///
    /// # Returns
    ///
    /// Returns a `Router` instance for the service.
    pub fn router<S>(&self) -> Router<S>
    where
        GithubOauthService: FromRef<S>,
        S: Clone + Send + Sync + 'static,
    {
        Router::<S>::new()
            .route("/login", get(login))
            .route("/authorize", get(authorize))
            .route("/logout", get(logout))
    }
}

/// Represents an action to perform after authentication.
pub enum AuthAction {
    /// Redirects to the specified path.
    Redirect(String),
    /// Represents an error that occurred during authentication.
    Error(Error),
}

impl IntoResponse for AuthAction {
    fn into_response(self) -> Response {
        match self {
            Self::Redirect(path) => Redirect::temporary(&path).into_response(),
            Self::Error(e) => ApiError::from(e).into_response(),
        }
    }
}

impl User {
    /// Creates a `User` instance from the headers and the GitHub OAuth service.
    ///
    /// # Arguments
    ///
    /// * `headers` - The headers containing the session cookie.
    /// * `state` - The GitHub OAuth service instance.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the `User` instance or an `AuthAction` if there was an error creating the user.
    pub fn from_headers_and_service(
        headers: &HeaderMap,
        service: &GithubOauthService,
    ) -> Result<Self, AuthAction> {
        let jar = PrivateCookieJar::from_headers(headers, service.config.session_key.clone());
        let session_cookie = jar
            .get(COOKIE_NAME)
            .ok_or(AuthAction::Redirect("/api/login".to_string()))?;

        let user: User = serde_json::from_str(session_cookie.value())
            .map_err(|e| AuthAction::Error(Error::DeserializeUser(e)))?;

        Ok(user)
    }
}

impl<S> FromRequestParts<S> for User
where
    GithubOauthService: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthAction;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let service = parts
            .extract_with_state(state)
            .await
            .map_err(|_| AuthAction::Error(Error::ServiceNotFound))?;

        User::from_headers_and_service(&parts.headers, &service)
    }
}

impl<S> OptionalFromRequestParts<S> for User
where
    GithubOauthService: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthAction;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        let service = parts
            .extract_with_state(state)
            .await
            .map_err(|_| AuthAction::Error(Error::ServiceNotFound))?;

        match User::from_headers_and_service(&parts.headers, &service) {
            Ok(user) => Ok(Some(user)),
            Err(AuthAction::Redirect(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
