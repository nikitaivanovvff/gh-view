use std::fmt;

pub const DEFAULT_GH_COMMAND_TIMEOUT_SECONDS: u64 = 30;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GhError {
    Missing(String),
    Unauthenticated(String),
    GitHubOutage(String),
    Timeout(String),
    Command(String),
}

impl GhError {
    pub fn from_command_output(message: String) -> Self {
        if is_auth_error(&message) {
            Self::Unauthenticated(message)
        } else if is_github_outage_error(&message) {
            Self::GitHubOutage(message)
        } else {
            Self::Command(message)
        }
    }
}

impl fmt::Display for GhError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(message)
            | Self::Unauthenticated(message)
            | Self::GitHubOutage(message)
            | Self::Timeout(message)
            | Self::Command(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for GhError {}

pub fn is_auth_error(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("gh auth login")
        || message.contains("not logged")
        || message.contains("authentication")
        || message.contains("http 401")
        || message.contains("requires authentication")
}

pub fn is_github_outage_error(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    ["http 500", "http 502", "http 503", "http 504"]
        .iter()
        .any(|needle| message.contains(needle))
        || message.contains("internal server error")
        || message.contains("bad gateway")
        || message.contains("service unavailable")
        || message.contains("gateway timeout")
        || message.contains("try again later")
}
