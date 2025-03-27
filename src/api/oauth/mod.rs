mod error;
mod github;
mod handlers;

pub use error::Error;
#[cfg_attr(test, allow(unused))]
pub(in crate::api) use github::GithubOauthService;

static CSRF_COOKIE_NAME: &str = "CSRF";
