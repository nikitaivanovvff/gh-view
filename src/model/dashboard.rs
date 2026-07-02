use super::PullRequest;
use std::collections::BTreeMap;
#[cfg(test)]
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Dashboard {
    pub my_prs: Vec<RepoGroup>,
    pub awaiting_review: Vec<RepoGroup>,
}

impl Dashboard {
    pub fn from_prs(my_prs: Vec<PullRequest>, awaiting_review: Vec<PullRequest>) -> Self {
        Self {
            my_prs: group_by_repo(my_prs),
            awaiting_review: group_by_repo(awaiting_review),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.my_prs.iter().all(|group| group.prs.is_empty())
            && self
                .awaiting_review
                .iter()
                .all(|group| group.prs.is_empty())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepoGroup {
    pub repo: String,
    pub prs: Vec<PullRequest>,
}

fn group_by_repo(prs: Vec<PullRequest>) -> Vec<RepoGroup> {
    let mut grouped: BTreeMap<String, Vec<PullRequest>> = BTreeMap::new();

    for pr in prs {
        grouped.entry(pr.repo.clone()).or_default().push(pr);
    }

    grouped
        .into_iter()
        .map(|(repo, mut prs)| {
            prs.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
            RepoGroup { repo, prs }
        })
        .collect()
}

#[cfg(test)]
pub fn repo_names(dashboard: &Dashboard) -> BTreeSet<String> {
    dashboard
        .my_prs
        .iter()
        .chain(dashboard.awaiting_review.iter())
        .map(|group| group.repo.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dashboard_is_empty() {
        assert!(Dashboard::default().is_empty());
    }

    #[test]
    fn groups_prs_by_repo() {
        let dashboard = Dashboard::from_prs(
            vec![
                pr("owner/b", 2, "2026-06-02"),
                pr("owner/a", 1, "2026-06-01"),
                pr("owner/b", 3, "2026-06-03"),
            ],
            vec![],
        );

        assert_eq!(dashboard.my_prs.len(), 2);
        assert_eq!(dashboard.my_prs[0].repo, "owner/a");
        assert_eq!(dashboard.my_prs[1].repo, "owner/b");
        assert_eq!(dashboard.my_prs[1].prs[0].number, 3);
    }

    #[test]
    fn reports_non_empty_when_any_section_has_prs() {
        assert!(!Dashboard::from_prs(vec![pr("owner/a", 1, "2026-06-01")], vec![]).is_empty());
        assert!(!Dashboard::from_prs(vec![], vec![pr("owner/a", 1, "2026-06-01")]).is_empty());
    }

    #[test]
    fn returns_unique_repo_names_across_sections() {
        let dashboard = Dashboard::from_prs(
            vec![pr("owner/a", 1, "2026-06-01")],
            vec![
                pr("owner/a", 2, "2026-06-02"),
                pr("owner/b", 3, "2026-06-03"),
            ],
        );

        assert_eq!(
            repo_names(&dashboard),
            ["owner/a".to_owned(), "owner/b".to_owned()].into()
        );
    }

    fn pr(repo: &str, number: u64, updated_at: &str) -> PullRequest {
        PullRequest {
            repo: repo.to_owned(),
            number,
            title: format!("PR {number}"),
            author: "author".to_owned(),
            url: format!("https://github.com/{repo}/pull/{number}"),
            updated_at: updated_at.to_owned(),
            state: "OPEN".to_owned(),
            is_draft: false,
            review_decision: None,
            check_status: None,
            reviewers: Vec::new(),
            review_requested: Vec::new(),
        }
    }
}
