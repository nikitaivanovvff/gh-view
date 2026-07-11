use anyhow::{Context, Result};

pub(super) const SEARCH_FIELDS: &str =
    "repository,number,title,author,headRefName,updatedAt,state,isDraft,url";
pub(super) const DETAIL_FIELDS: &str = "number,title,author,updatedAt,isDraft,url,body,state,mergeable,headRefName,baseRefName,reviewDecision,statusCheckRollup,comments,reviews";
pub(super) fn review_threads_query(cursor: Option<&str>) -> String {
    let after = cursor_argument(cursor);
    format!(
        r#"
query($owner: String!, $name: String!, $number: Int!) {{
  repository(owner: $owner, name: $name) {{
    pullRequest(number: $number) {{
      reviewThreads(first: 100{after}) {{
        nodes {{
          isResolved
          path
          line
          originalLine
          # Nested connections cannot be independently paginated in this bulk query.
          comments(first: 100) {{
            nodes {{
              author {{ login }}
              body
              diffHunk
              createdAt
              url
            }}
          }}
        }}
        pageInfo {{ hasNextPage endCursor }}
      }}
    }}
  }}
}}
"#
    )
}

pub(super) fn dashboard_search_query(query: &str, cursor: Option<&str>) -> String {
    let escaped_query = escape_graphql_string(query);
    let after = cursor_argument(cursor);
    format!(
        r#"{{
  search(query: "{escaped_query}", type: ISSUE, first: 100{after}) {{
    nodes {{
      ...DashboardPullRequestFields
    }}
    pageInfo {{ hasNextPage endCursor }}
  }}
}}

{DASHBOARD_PULL_REQUEST_FRAGMENT}"#
    )
}

pub(super) fn dashboard_query(
    login: &str,
    my_cursor: Option<Option<&str>>,
    review_cursor: Option<Option<&str>>,
) -> String {
    let my_query = escape_graphql_string(&format!("is:pr is:open author:{login} archived:false"));
    let review_query = escape_graphql_string(&format!(
        "is:pr is:open review-requested:{login} archived:false"
    ));
    let my_prs = my_cursor.map_or_else(String::new, |cursor| {
        let after = cursor_argument(cursor);
        format!(
            r#"  myPrs: search(query: "{my_query}", type: ISSUE, first: 100{after}) {{
    nodes {{ ...DashboardPullRequestFields }}
    pageInfo {{ hasNextPage endCursor }}
  }}"#
        )
    });
    let review_requests = review_cursor.map_or_else(String::new, |cursor| {
        let after = cursor_argument(cursor);
        format!(
            r#"  reviewRequests: search(query: "{review_query}", type: ISSUE, first: 100{after}) {{
    nodes {{ ...DashboardPullRequestFields }}
    pageInfo {{ hasNextPage endCursor }}
  }}"#
        )
    });
    format!(
        r#"{{
{my_prs}
{review_requests}
}}

{DASHBOARD_PULL_REQUEST_FRAGMENT}"#
    )
}

fn cursor_argument(cursor: Option<&str>) -> String {
    cursor.map_or_else(String::new, |cursor| {
        format!(r#", after: "{}""#, escape_graphql_string(cursor))
    })
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
  # These nested connections cannot be independently paginated in a search result.
  reviews(last: 100) {
    nodes {
      author { login __typename }
      state
    }
  }
  reviewRequests(first: 100) {
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

        let query = dashboard_search_query(r#"author:octo\"cat"#, None);
        assert!(query.contains(r#"author:octo\\\"cat"#));
        assert!(query.contains("DashboardPullRequestFields"));
    }

    #[test]
    fn dashboard_query_builds_both_dashboard_sections() {
        let query = dashboard_query(r#"octo\"cat"#, Some(None), Some(None));

        assert!(query.contains("myPrs: search"));
        assert!(query.contains("reviewRequests: search"));
        assert!(query.contains(r#"author:octo\\\"cat"#));
        assert!(query.contains(r#"review-requested:octo\\\"cat"#));
    }

    #[test]
    fn queries_include_independent_cursor_arguments() {
        let query = dashboard_query("octocat", Some(Some("mine")), Some(Some("reviews")));
        assert!(query.contains(r#"myPrs: search(query: "is:pr is:open author:octocat archived:false", type: ISSUE, first: 100, after: "mine")"#));
        assert!(query.contains(r#"reviewRequests: search(query: "is:pr is:open review-requested:octocat archived:false", type: ISSUE, first: 100, after: "reviews")"#));

        let query = dashboard_query("octocat", None, Some(Some("next")));
        assert!(!query.contains("myPrs: search"));
        assert!(query.contains(r#"after: "next""#));
        assert!(
            dashboard_search_query("is:pr", Some("search-next"))
                .contains(r#"after: "search-next""#)
        );
        assert!(review_threads_query(Some("thread-next")).contains(r#"after: "thread-next""#));
    }

    #[test]
    fn splits_repo_names() {
        assert_eq!(split_repo("owner/name").unwrap(), ("owner", "name"));
        assert!(split_repo("owner").is_err());
        assert!(split_repo("owner/").is_err());
    }
}
