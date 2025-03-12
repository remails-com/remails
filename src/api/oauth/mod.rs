mod error;
mod github;
mod handlers;

pub use error::Error;
#[cfg_attr(test, allow(unused))]
pub use github::{GithubOauthService, User};

static COOKIE_NAME: &str = "SESSION";
static CSRF_COOKIE_NAME: &str = "CSRF";
static USER_AGENT_VALUE: &str = "remails";
