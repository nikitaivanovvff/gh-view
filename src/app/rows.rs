use crate::model::{Dashboard, PullRequest, RepoGroup};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
pub(super) const DEFAULT_PRS_PER_REPO_PAGE: usize = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DashboardSection {
    MyPrs,
    AwaitingReview,
}

#[derive(Clone, Debug)]
pub enum Row<'a> {
    Section(&'static str),
    Group {
        section: DashboardSection,
        repo: &'a str,
        count: usize,
        open: bool,
        page: usize,
        page_count: usize,
    },
    Pr {
        section: DashboardSection,
        pr: &'a PullRequest,
    },
    Message(String),
}

impl Row<'_> {
    pub(super) fn group_key(&self) -> Option<String> {
        match self {
            Row::Group { section, repo, .. } => Some(group_key(*section, repo)),
            _ => None,
        }
    }

    pub(super) fn pr_url(&self) -> Option<&str> {
        match self {
            Row::Pr { pr, .. } => Some(&pr.url),
            _ => None,
        }
    }

    pub(super) fn pr(&self) -> Option<&PullRequest> {
        match self {
            Row::Pr { pr, .. } => Some(pr),
            _ => None,
        }
    }
}

pub(super) fn push_groups<'a>(
    rows: &mut Vec<Row<'a>>,
    section: DashboardSection,
    groups: &'a [RepoGroup],
    collapsed: &BTreeSet<String>,
    pages: &BTreeMap<String, usize>,
    page_size: usize,
) {
    if groups.is_empty() {
        rows.push(Row::Message("  none".to_owned()));
        return;
    }

    for group in groups {
        let key = group_key(section, &group.repo);
        let open = !collapsed.contains(&key);
        let page_count = page_count(group.prs.len(), page_size);
        let page = pages
            .get(&key)
            .copied()
            .unwrap_or_default()
            .min(page_count.saturating_sub(1));
        rows.push(Row::Group {
            section,
            repo: &group.repo,
            count: group.prs.len(),
            open,
            page: page + 1,
            page_count,
        });

        if open {
            let start = page * page_size;
            let end = (start + page_size).min(group.prs.len());
            rows.extend(
                group.prs[start..end]
                    .iter()
                    .map(|pr| Row::Pr { section, pr }),
            );
        }
    }
}

pub(super) fn page_count(pr_count: usize, page_size: usize) -> usize {
    pr_count.div_ceil(page_size.max(1)).max(1)
}

pub(super) fn group_names(dashboard: &Dashboard) -> BTreeSet<String> {
    dashboard
        .my_prs
        .iter()
        .map(|group| group_key(DashboardSection::MyPrs, &group.repo))
        .chain(
            dashboard
                .awaiting_review
                .iter()
                .map(|group| group_key(DashboardSection::AwaitingReview, &group.repo)),
        )
        .collect()
}

pub(super) fn group_key(section: DashboardSection, repo: &str) -> String {
    let prefix = match section {
        DashboardSection::MyPrs => "my",
        DashboardSection::AwaitingReview => "review",
    };
    format!("{prefix}:{repo}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_groups_adds_placeholder_for_empty_sections() {
        let mut rows = Vec::new();

        push_groups(
            &mut rows,
            DashboardSection::MyPrs,
            &[],
            &BTreeSet::new(),
            &BTreeMap::new(),
            DEFAULT_PRS_PER_REPO_PAGE,
        );

        assert!(matches!(rows.as_slice(), [Row::Message(message)] if message == "  none"));
    }

    #[test]
    fn push_groups_includes_prs_only_for_expanded_groups() {
        let groups = vec![RepoGroup {
            repo: "owner/repo".to_owned(),
            prs: vec![pr("owner/repo", 1), pr("owner/repo", 2)],
        }];
        let mut collapsed = BTreeSet::new();
        collapsed.insert("my:owner/repo".to_owned());
        let mut collapsed_rows = Vec::new();
        let mut expanded_rows = Vec::new();

        push_groups(
            &mut collapsed_rows,
            DashboardSection::MyPrs,
            &groups,
            &collapsed,
            &BTreeMap::new(),
            DEFAULT_PRS_PER_REPO_PAGE,
        );
        push_groups(
            &mut expanded_rows,
            DashboardSection::AwaitingReview,
            &groups,
            &collapsed,
            &BTreeMap::new(),
            DEFAULT_PRS_PER_REPO_PAGE,
        );

        assert!(matches!(
            collapsed_rows.as_slice(),
            [Row::Group {
                open: false,
                count: 2,
                ..
            }]
        ));
        assert!(matches!(
            expanded_rows.as_slice(),
            [
                Row::Group {
                    open: true,
                    count: 2,
                    ..
                },
                Row::Pr { .. },
                Row::Pr { .. }
            ]
        ));
    }

    #[test]
    fn push_groups_limits_expanded_prs_to_selected_repo_page() {
        let groups = vec![RepoGroup {
            repo: "owner/repo".to_owned(),
            prs: (1..=7).map(|number| pr("owner/repo", number)).collect(),
        }];
        let mut pages = BTreeMap::new();
        pages.insert("my:owner/repo".to_owned(), 1);
        let mut rows = Vec::new();

        push_groups(
            &mut rows,
            DashboardSection::MyPrs,
            &groups,
            &BTreeSet::new(),
            &pages,
            DEFAULT_PRS_PER_REPO_PAGE,
        );

        assert!(matches!(
            rows.first(),
            Some(Row::Group {
                page: 2,
                page_count: 3,
                ..
            })
        ));
        let numbers: Vec<_> = rows
            .iter()
            .filter_map(|row| row.pr().map(|pr| pr.number))
            .collect();
        assert_eq!(numbers, vec![4, 5, 6]);
    }

    #[test]
    fn group_names_are_namespaced_by_section() {
        let dashboard = Dashboard {
            my_prs: vec![RepoGroup {
                repo: "owner/shared".to_owned(),
                prs: vec![pr("owner/shared", 1)],
            }],
            awaiting_review: vec![RepoGroup {
                repo: "owner/shared".to_owned(),
                prs: vec![pr("owner/shared", 2)],
            }],
        };

        assert_eq!(
            group_names(&dashboard),
            [
                "my:owner/shared".to_owned(),
                "review:owner/shared".to_owned()
            ]
            .into()
        );
    }

    #[test]
    fn row_accessors_return_values_for_matching_variants_only() {
        let pr = pr("owner/repo", 1);
        let group = Row::Group {
            section: DashboardSection::AwaitingReview,
            repo: "owner/repo",
            count: 1,
            open: true,
            page: 1,
            page_count: 1,
        };
        let row = Row::Pr {
            section: DashboardSection::MyPrs,
            pr: &pr,
        };
        let message = Row::Message("empty".to_owned());

        assert_eq!(group.group_key().as_deref(), Some("review:owner/repo"));
        assert_eq!(row.pr_url(), Some("https://github.com/owner/repo/pull/1"));
        assert_eq!(row.pr().map(|pr| pr.number), Some(1));
        assert_eq!(message.group_key(), None);
        assert_eq!(message.pr_url(), None);
        assert!(message.pr().is_none());
    }

    fn pr(repo: &str, number: u64) -> PullRequest {
        PullRequest {
            repo: repo.to_owned(),
            number,
            title: format!("PR {number}"),
            author: "author".to_owned(),
            head_ref: format!("feature-{number}"),
            url: format!("https://github.com/{repo}/pull/{number}"),
            updated_at: "2026-07-01T10:00:00Z".to_owned(),
            state: "OPEN".to_owned(),
            is_draft: false,
            review_decision: None,
            check_status: None,
            reviewers: Vec::new(),
            review_requested: Vec::new(),
        }
    }
}
