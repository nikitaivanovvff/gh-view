use super::pull_request_status;
use super::rows::DashboardSection;
use crate::model::{CheckStatus, Dashboard, PullRequest, RepoGroup};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DashboardSearchState {
    pub query: String,
    pub selected: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DashboardSearchMatch {
    pub pr: PullRequest,
    pub sections: Vec<DashboardSection>,
    pub match_reason: Option<SearchMatchReason>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchMatchReason {
    pub label: &'static str,
    pub value: String,
}

struct SearchCandidate<'a> {
    pr: &'a PullRequest,
    sections: Vec<DashboardSection>,
    text: String,
    order: usize,
}

pub(super) fn search_matches(dashboard: &Dashboard, query: &str) -> Vec<DashboardSearchMatch> {
    let candidates = search_candidates(dashboard);
    let query = query.trim();

    if query.is_empty() {
        return candidates
            .into_iter()
            .map(|candidate| candidate.into_match(None))
            .collect();
    }

    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut haystack = Vec::new();

    let mut scored = candidates
        .into_iter()
        .filter_map(|candidate| {
            let score = pattern.score(
                Utf32Str::new(candidate.text.as_str(), &mut haystack),
                &mut matcher,
            )?;
            haystack.clear();
            let reason = best_match_reason(&pattern, &candidate, &mut matcher, &mut haystack);
            Some((score, candidate.order, candidate, reason))
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    scored
        .into_iter()
        .map(|(_, _, candidate, reason)| candidate.into_match(reason))
        .collect()
}

fn search_candidates(dashboard: &Dashboard) -> Vec<SearchCandidate<'_>> {
    let mut candidates = Vec::new();
    let mut identities = HashMap::new();
    push_section_candidates(
        &mut candidates,
        &mut identities,
        DashboardSection::MyPrs,
        &dashboard.my_prs,
    );
    push_section_candidates(
        &mut candidates,
        &mut identities,
        DashboardSection::AwaitingReview,
        &dashboard.awaiting_review,
    );
    candidates
}

fn push_section_candidates<'a>(
    candidates: &mut Vec<SearchCandidate<'a>>,
    identities: &mut HashMap<(&'a str, u64), usize>,
    section: DashboardSection,
    groups: &'a [RepoGroup],
) {
    for group in groups {
        for pr in &group.prs {
            let identity = (pr.repo.as_str(), pr.number);
            if let Some(index) = identities.get(&identity).copied() {
                let candidate = &mut candidates[index];
                if !candidate.sections.contains(&section) {
                    candidate.sections.push(section);
                    candidate.text.push(' ');
                    candidate.text.push_str(section.title());
                }
                continue;
            }
            identities.insert(identity, candidates.len());
            candidates.push(SearchCandidate {
                pr,
                sections: vec![section],
                text: candidate_text(section.title(), pr),
                order: candidates.len(),
            });
        }
    }
}

fn candidate_text(section: &str, pr: &PullRequest) -> String {
    let reviewers = pr
        .reviewers
        .iter()
        .map(|reviewer| reviewer.login.as_str())
        .chain(pr.review_requested.iter().map(|target| target.name()))
        .collect::<Vec<_>>()
        .join(" ");
    let check_status = pr.check_status.as_ref().map_or("", CheckStatus::label);
    let review_status = pull_request_status(pr);

    format!(
        "{} #{} {} {} {} {} {} {} {}",
        pr.repo,
        pr.number,
        pr.title,
        pr.head_ref,
        pr.author,
        reviewers,
        review_status,
        check_status,
        section
    )
}

fn best_match_reason(
    pattern: &Pattern,
    candidate: &SearchCandidate<'_>,
    matcher: &mut Matcher,
    haystack: &mut Vec<char>,
) -> Option<SearchMatchReason> {
    search_fields(candidate)
        .into_iter()
        .filter_map(|field| {
            let score = pattern.score(Utf32Str::new(&field.value, haystack), matcher);
            haystack.clear();
            score.map(|score| (score, field))
        })
        .max_by_key(|(score, _)| *score)
        .and_then(|(_, field)| {
            (!field.visible).then_some(SearchMatchReason {
                label: field.label,
                value: field.value,
            })
        })
}

struct SearchField {
    label: &'static str,
    value: String,
    visible: bool,
}

fn search_fields(candidate: &SearchCandidate<'_>) -> Vec<SearchField> {
    let pr = candidate.pr;
    let mut fields = vec![
        SearchField {
            label: "repository",
            value: pr.repo.clone(),
            visible: true,
        },
        SearchField {
            label: "number",
            value: format!("#{}", pr.number),
            visible: true,
        },
        SearchField {
            label: "title",
            value: pr.title.clone(),
            visible: true,
        },
        SearchField {
            label: "branch",
            value: pr.head_ref.clone(),
            visible: true,
        },
        SearchField {
            label: "author",
            value: format!("@{}", pr.author),
            visible: false,
        },
        SearchField {
            label: "status",
            value: pull_request_status(pr),
            visible: true,
        },
    ];
    if let Some(status) = pr.check_status.as_ref() {
        fields.push(SearchField {
            label: "CI",
            value: status.label().to_owned(),
            visible: false,
        });
    }
    fields.extend(pr.reviewers.iter().map(|reviewer| SearchField {
        label: "reviewer",
        value: format!("@{}", reviewer.login),
        visible: false,
    }));
    fields.extend(pr.review_requested.iter().map(|target| SearchField {
        label: "requested",
        value: format!("@{}", target.name()),
        visible: false,
    }));
    fields.extend(candidate.sections.iter().map(|section| SearchField {
        label: "section",
        value: section.title().to_owned(),
        visible: true,
    }));
    fields
}

impl SearchCandidate<'_> {
    fn into_match(self, match_reason: Option<SearchMatchReason>) -> DashboardSearchMatch {
        DashboardSearchMatch {
            pr: self.pr.clone(),
            sections: self.sections,
            match_reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CheckStatus, ReviewRequestTarget, Reviewer, ReviewerState};

    #[test]
    fn empty_query_returns_all_prs_in_dashboard_order() {
        let dashboard = Dashboard::from_prs(
            vec![pr("owner/a", 1, "Alpha", "alice")],
            vec![pr("owner/b", 2, "Beta", "bob")],
        );

        let matches = search_matches(&dashboard, "");

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].pr.number, 1);
        assert_eq!(matches[0].sections, vec![DashboardSection::MyPrs]);
        assert_eq!(matches[0].match_reason, None);
        assert_eq!(matches[1].pr.number, 2);
        assert_eq!(matches[1].sections, vec![DashboardSection::AwaitingReview]);
    }

    #[test]
    fn query_matches_candidate_fields() {
        let mut first = pr("owner/api", 42, "Add retry budget", "alice");
        first.reviewers = vec![Reviewer {
            login: "carol".to_owned(),
            state: ReviewerState::Approved,
        }];
        first.review_requested = vec![ReviewRequestTarget::User("dave".to_owned())];
        first.review_decision = Some("APPROVED".to_owned());
        first.check_status = Some(CheckStatus::Passing);
        let dashboard = Dashboard::from_prs(vec![first], vec![pr("owner/web", 7, "Navbar", "bob")]);

        for query in [
            "owner/api",
            "#42",
            "retry budget",
            "feature/add",
            "alice",
            "carol",
            "dave",
            "approved",
            "passing",
            "my prs",
        ] {
            let matches = search_matches(&dashboard, query);
            assert_eq!(
                matches.first().map(|item| item.pr.number),
                Some(42),
                "{query}"
            );
        }
    }

    #[test]
    fn hidden_search_fields_explain_why_a_pr_matched() {
        let mut item = pr("owner/api", 42, "Visible title", "hidden-author");
        item.reviewers = vec![Reviewer {
            login: "needle-reviewer".to_owned(),
            state: ReviewerState::Commented,
        }];
        let dashboard = Dashboard::from_prs(vec![item], vec![]);

        let author = &search_matches(&dashboard, "hidden-author")[0];
        assert_eq!(author.match_reason.as_ref().unwrap().label, "author");
        assert_eq!(
            author.match_reason.as_ref().unwrap().value,
            "@hidden-author"
        );

        let reviewer = &search_matches(&dashboard, "needle-reviewer")[0];
        assert_eq!(reviewer.match_reason.as_ref().unwrap().label, "reviewer");
        assert_eq!(
            reviewer.match_reason.as_ref().unwrap().value,
            "@needle-reviewer"
        );

        assert_eq!(
            search_matches(&dashboard, "Visible title")[0].match_reason,
            None
        );
    }

    #[test]
    fn matching_is_case_insensitive_and_returns_no_matches() {
        let dashboard = Dashboard::from_prs(vec![pr("owner/api", 1, "Add Retry", "alice")], vec![]);

        assert_eq!(search_matches(&dashboard, "RETRY").len(), 1);
        assert!(search_matches(&dashboard, "zzzzzz").is_empty());
    }

    #[test]
    fn review_requests_section_name_is_searchable() {
        let dashboard = Dashboard::from_prs(vec![], vec![pr("owner/api", 1, "Add Retry", "alice")]);

        assert_eq!(
            search_matches(&dashboard, "review requests")
                .first()
                .map(|item| item.pr.number),
            Some(1)
        );
    }

    #[test]
    fn deduplicates_prs_across_sections_and_preserves_memberships() {
        let shared = pr("owner/api", 1, "Add Retry", "alice");
        let dashboard = Dashboard::from_prs(vec![shared.clone()], vec![shared]);

        let matches = search_matches(&dashboard, "");

        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].sections,
            vec![DashboardSection::MyPrs, DashboardSection::AwaitingReview]
        );
        assert_eq!(search_matches(&dashboard, "my prs").len(), 1);
        assert_eq!(search_matches(&dashboard, "review requests").len(), 1);
    }

    fn pr(repo: &str, number: u64, title: &str, author: &str) -> PullRequest {
        PullRequest {
            repo: repo.to_owned(),
            number,
            title: title.to_owned(),
            author: author.to_owned(),
            head_ref: "feature/add".to_owned(),
            url: format!("https://github.com/{repo}/pull/{number}"),
            updated_at: format!("2026-07-{number:02}T10:00:00Z"),
            state: "OPEN".to_owned(),
            is_draft: false,
            review_decision: None,
            check_status: None,
            reviewers: Vec::new(),
            review_requested: Vec::new(),
        }
    }
}
