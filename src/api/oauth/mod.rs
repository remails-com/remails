mod error;
mod github;
mod handlers;

use crate::models::ApiUser;
pub use error::Error;
#[cfg_attr(test, allow(unused))]
pub(in crate::api) use github::GithubOauthService;
use oauth2::{
    AccessToken, Client, EndpointNotSet, EndpointSet, Scope, StandardRevocableToken,
    basic::{
        BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
        BasicTokenResponse,
    },
};

static CSRF_COOKIE_NAME: &str = "CSRF";

trait OAuthService {
    fn scopes() -> Vec<Scope>;
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
    >;

    async fn fetch_user(
        &self,
        token: &AccessToken,
        logged_in_user: Option<ApiUser>,
    ) -> Result<ApiUser, Error>;
}
