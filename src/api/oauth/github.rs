use super::handlers::{authorize, oauth_login};
use crate::{
    api::{
        ApiState, USER_AGENT_VALUE,
        oauth::{Error, OAuthService},
        whoami::WhoamiResponse,
    },
    models::{self, ApiUser, ApiUserRepository, NewApiUser, RuntimeConfigRepository},
};
use axum::{Json, Router, extract::State, routing::get};
use http::{
    HeaderValue,
    header::{ACCEPT, USER_AGENT},
};
use oauth2::{
    AccessToken, AuthUrl, Client, ClientId, ClientSecret, EndpointNotSet, EndpointSet, RedirectUrl,
    Scope, StandardRevocableToken, TokenUrl,
    basic::{
        BasicClient, BasicErrorResponse, BasicRevocationErrorResponse,
        BasicTokenIntrospectionResponse, BasicTokenResponse,
    },
};
use reqwest::redirect::Policy;
use serde::{Deserialize, de::DeserializeOwned};
use sqlx::PgPool;
use std::{env, fmt::Debug, time::Duration};
use tracing::{debug, trace};

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
    http_client: reqwest::Client,
    user_repository: ApiUserRepository,
    config_repository: RuntimeConfigRepository,
}

#[derive(Debug, Deserialize)]
struct GitHubUser {
    pub name: Option<String>,
    pub login: String,
    pub id: i64,
}

#[derive(Debug, Deserialize)]
struct GitHubEmail {
    email: String,
    verified: bool,
    primary: bool,
}

pub async fn disconnect_github(
    State(user_repository): State<ApiUserRepository>,
    api_user: ApiUser,
) -> Result<Json<WhoamiResponse>, Error> {
    if !api_user.password_enabled() {
        Err(Error::Conflict(
            "Password-based login is not enabled".to_string(),
        ))?;
    };

    user_repository.remove_github_id(api_user.id()).await?;

    Ok(Json(WhoamiResponse::logged_in(
        user_repository
            .find_by_id(api_user.id())
            .await
            .transpose()
            .ok_or(models::Error::NotFound(
                "Could not find user to disconnect from GitHub",
            ))??,
    )))
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
    pub fn new(pool: PgPool) -> Result<Self, Error> {
        let client_id = env::var("OAUTH_CLIENT_ID")
            .map_err(|_| Error::MissingEnvironmentVariable("OAUTH_CLIENT_ID"))?;
        let client_secret = env::var("OAUTH_CLIENT_SECRET")
            .map_err(|_| Error::MissingEnvironmentVariable("OAUTH_CLIENT_SECRET"))?;
        let redirect_url = env::var("GITHUB_OAUTH_REDIRECT_URL")
            .map_err(|_| Error::MissingEnvironmentVariable("GITHUB_OAUTH_REDIRECT_URL"))?
            .parse()
            .expect("Failed to parse GITHUB_OAUTH_REDIRECT_URL");

        let oauth_client = BasicClient::new(ClientId::new(client_id))
            .set_client_secret(ClientSecret::new(client_secret))
            .set_auth_uri(AuthUrl::from_url(GITHUB_AUTH_URL.parse().unwrap()))
            .set_token_uri(TokenUrl::from_url(GITHUB_TOKEN_URL.parse().unwrap()))
            .set_redirect_uri(RedirectUrl::from_url(redirect_url));

        let http_client = reqwest::Client::builder()
            .use_rustls_tls()
            .redirect(Policy::none())
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|e| Error::FetchUser(e.to_string()))?;

        Ok(Self {
            oauth_client,
            http_client,
            user_repository: ApiUserRepository::new(pool.clone()),
            config_repository: RuntimeConfigRepository::new(pool),
        })
    }

    /// Creates a router for the GitHub OAuth service.
    ///
    /// # Returns
    ///
    /// Returns a `Router` instance for the service.
    pub fn router(&self) -> Router<ApiState> {
        Router::new()
            .route(
                "/api/login/github",
                get(oauth_login::<GithubOauthService>).delete(disconnect_github),
            )
            .route(
                "/api/oauth/authorize/github",
                get(authorize::<GithubOauthService>),
            )
    }

    async fn github_sign_up(
        &self,
        github_user: GitHubUser,
        token: &AccessToken,
    ) -> Result<ApiUser, Error> {
        if !self.config_repository.account_creation_is_enabled().await? {
            return Err(Error::Forbidden);
        }

        // Fetch email addresses from the GitHub API
        let emails: Vec<GitHubEmail> = self.fetch_gh_api(GITHUB_EMAILS_URL, token).await?;

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

        Ok(self
            .user_repository
            .create(NewApiUser {
                email,
                name: github_user.name.unwrap_or(github_user.login),
                password: None,
                global_role: None,
                org_roles: vec![],
                github_user_id: Some(github_user.id),
            })
            .await?)
    }

    async fn fetch_gh_api<T>(&self, url: &str, token: &AccessToken) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        self.http_client
            .get(url)
            .header(ACCEPT, HeaderValue::from_static(GITHUB_ACCEPT_TYPE))
            .header(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE))
            .bearer_auth(token.secret())
            .send()
            .await
            .map_err(|e| Error::FetchUser(e.to_string()))?
            .json()
            .await
            .map_err(|e| Error::ParseUser(e.to_string()))
    }
}

impl OAuthService for GithubOauthService {
    fn scopes() -> Vec<Scope> {
        vec![
            Scope::new("read:user".to_string()),
            Scope::new("user:email".to_string()),
        ]
    }

    fn oauth_client(
        &self,
    ) -> &Client<
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
    > {
        &self.oauth_client
    }

    async fn fetch_user(
        &self,
        token: &AccessToken,
        logged_in_user: Option<ApiUser>,
    ) -> Result<ApiUser, Error> {
        // Fetch user data from the GitHub API
        let user_data: GitHubUser = self.fetch_gh_api(GITHUB_USER_URL, token).await?;

        // Add the GitHub ID as an additional login method to an existing API user
        if let Some(logged_in_user) = logged_in_user {
            debug!(
                user_id = logged_in_user.id().to_string(),
                "GitHub ID {} as login method for user '{}'", user_data.id, logged_in_user.name
            );
            self.user_repository
                .add_github_id(logged_in_user.id(), user_data.id)
                .await?;
        }

        if let Some(existing_user) = self.user_repository.find_by_github_id(user_data.id).await? {
            trace!(
                user_id = existing_user.id().to_string(),
                "Signed in with GitHub for existing user"
            );
            Ok(existing_user)
        } else {
            let new = self.github_sign_up(user_data, token).await?;
            debug!(
                user_id = new.id().to_string(),
                "Signed up new user via GitHub"
            );
            Ok(new)
        }
    }
}
