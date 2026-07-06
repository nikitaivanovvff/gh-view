use anyhow::{Context, Result};

pub(super) const SEARCH_FIELDS: &str =
    "repository,number,title,author,headRefName,updatedAt,state,isDraft,url";
pub(super) const DETAIL_FIELDS: &str = "number,title,author,updatedAt,isDraft,url,body,state,mergeable,headRefName,baseRefName,reviewDecision,statusCheckRollup,comments,reviews";
pub(super) const REVIEW_THREADS_QUERY: &str = r#"
query($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      reviewThreads(first: 50) {
        nodes {
          isResolved
          path
          line
          originalLine
          comments(first: 50) {
            nodes {
              author { login }
              body
              diffHunk
              createdAt
              url
            }
          }
        }
      }
    }
  }
}
"#;

pub(super) fn dashboard_search_query(query: &str) -> String {
    let escaped_query = escape_graphql_string(query);
    format!(
        r#"{{
  search(query: "{escaped_query}", type: ISSUE, first: 50) {{
    nodes {{
      ...DashboardPullRequestFields
    }}
  }}
}}

{DASHBOARD_PULL_REQUEST_FRAGMENT}"#
    )
}

pub(super) fn dashboard_query(login: &str) -> String {
    let my_query = escape_graphql_string(&format!("is:pr is:open author:{login} archived:false"));
    let review_query = escape_graphql_string(&format!(
        "is:pr is:open review-requested:{login} archived:false"
    ));
    format!(
        r#"{{
  myPrs: search(query: "{my_query}", type: ISSUE, first: 50) {{
    nodes {{
      ...DashboardPullRequestFields
    }}
  }}
  reviewRequests: search(query: "{review_query}", type: ISSUE, first: 50) {{
    nodes {{
      ...DashboardPullRequestFields
    }}
  }}
}}

{DASHBOARD_PULL_REQUEST_FRAGMENT}"#
    )
}

pub(super) fn split_repo(repo: &str) -> Result<(&str, &str)> {
    repo.split_once('/')
        .filter(|(owner, name)| !owner.is_empty() && !name.is_empty())
        .context("repository name must be in owner/name format")
}

fn escape_graphql_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

const DASHBOARD_PULL_REQUEST_FRAGMENT: &str = r#"
fragment DashboardPullRequestFields on PullRequest {
  repository { nameWithOwner }
  number
  title
  url
  isDraft
  reviewDecision
  updatedAt
  author { login }
  headRefName
  reviews(last: 20) {
    nodes {
      author { login __typename }
      state
    }
  }
  reviewRequests(first: 20) {
    nodes {
      requestedReviewer {
        ... on User { login __typename }
        ... on Team { name __typename }
      }
    }
  }
  commits(last: 1) {
    nodes {
      commit {
        statusCheckRollup { state }
      }
    }
  }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphql_string_escaping_handles_quotes_and_backslashes() {
        assert_eq!(escape_graphql_string(r#"owner\"repo"#), r#"owner\\\"repo"#);

        let query = dashboard_search_query(r#"author:octo\"cat"#);
        assert!(query.contains(r#"author:octo\\\"cat"#));
        assert!(query.contains("DashboardPullRequestFields"));
    }

    #[test]
    fn dashboard_query_builds_both_dashboard_sections() {
        let query = dashboard_query(r#"octo\"cat"#);

        assert!(query.contains("myPrs: search"));
        assert!(query.contains("reviewRequests: search"));
        assert!(query.contains(r#"author:octo\\\"cat"#));
        assert!(query.contains(r#"review-requested:octo\\\"cat"#));
    }

    #[test]
    fn splits_repo_names() {
        assert_eq!(split_repo("owner/name").unwrap(), ("owner", "name"));
        assert!(split_repo("owner").is_err());
        assert!(split_repo("owner/").is_err());
    }
}
