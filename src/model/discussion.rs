#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscussionItem {
    pub kind: DiscussionKind,
    pub author: String,
    pub body: String,
    pub created_at: String,
    pub url: String,
    pub replies: Vec<DiscussionReply>,
    pub code_context: Option<CodeContext>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiscussionKind {
    IssueComment,
    ReviewThread { resolved: bool },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscussionReply {
    pub author: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeContext {
    pub path: String,
    pub start_line: Option<u64>,
    pub highlighted_line: Option<u64>,
    pub highlighted_kind: Option<CodeLineKind>,
    pub lines: Vec<CodeContextLine>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeContextLine {
    pub number: Option<u64>,
    pub kind: CodeLineKind,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodeLineKind {
    Context,
    Added,
    Removed,
}
