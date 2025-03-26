mod error;
mod github;
mod handlers;

pub use error::Error;
#[cfg_attr(test, allow(unused))]
pub use github::GithubOauthService;

static CSRF_COOKIE_NAME: &str = "CSRF";
