use crate::model::{CodeContextLine, CodeLineKind};

const CONTEXT_RADIUS: u64 = 10;

pub(super) fn source_context_lines(
    text: &str,
    highlighted_line: u64,
    patch: Option<&str>,
) -> Vec<CodeContextLine> {
    let start = highlighted_line.saturating_sub(CONTEXT_RADIUS).max(1);
    let end = highlighted_line.saturating_add(CONTEXT_RADIUS);

    let mut lines: Vec<CodeContextLine> = text
        .lines()
        .enumerate()
        .filter_map(|(index, text)| {
            let number = index as u64 + 1;
            (number >= start && number <= end).then(|| CodeContextLine {
                number: Some(number),
                kind: CodeLineKind::Context,
                text: text.to_owned(),
            })
        })
        .collect();

    if let Some(patch) = patch {
        apply_patch_kinds(&mut lines, patch);
    }

    lines
}

fn apply_patch_kinds(lines: &mut Vec<CodeContextLine>, patch: &str) {
    let mut old_line = None;
    let mut new_line = None;
    let mut removed_lines = Vec::new();

    for raw_line in patch.lines() {
        if raw_line.starts_with("@@") {
            flush_removed_lines(lines, new_line, &mut removed_lines);
            if let Some((old_start, new_start)) = parse_patch_hunk_header(raw_line) {
                old_line = Some(old_start);
                new_line = Some(new_start);
            }
            continue;
        }

        let Some(prefix) = raw_line.chars().next() else {
            continue;
        };
        match prefix {
            '+' => {
                flush_removed_lines(lines, new_line, &mut removed_lines);
                if let Some(number) = new_line {
                    mark_line_kind(lines, number, &raw_line[1..], CodeLineKind::Added);
                }
                new_line = new_line.map(|line| line + 1);
            }
            '-' => {
                if let Some(number) = old_line {
                    removed_lines.push(CodeContextLine {
                        number: Some(number),
                        kind: CodeLineKind::Removed,
                        text: raw_line[1..].to_owned(),
                    });
                }
                old_line = old_line.map(|line| line + 1);
            }
            ' ' => {
                flush_removed_lines(lines, new_line, &mut removed_lines);
                old_line = old_line.map(|line| line + 1);
                new_line = new_line.map(|line| line + 1);
            }
            _ => {
                flush_removed_lines(lines, new_line, &mut removed_lines);
                old_line = old_line.map(|line| line + 1);
                new_line = new_line.map(|line| line + 1);
            }
        }
    }

    flush_removed_lines(lines, new_line, &mut removed_lines);
}

fn flush_removed_lines(
    lines: &mut Vec<CodeContextLine>,
    before_number: Option<u64>,
    removed_lines: &mut Vec<CodeContextLine>,
) {
    if removed_lines.is_empty() {
        return;
    }
    let Some(before_number) = before_number else {
        removed_lines.clear();
        return;
    };
    let Some(index) = lines
        .iter()
        .position(|line| line.number == Some(before_number))
    else {
        removed_lines.clear();
        return;
    };

    lines.splice(index..index, removed_lines.drain(..));
}

fn mark_line_kind(lines: &mut [CodeContextLine], number: u64, text: &str, kind: CodeLineKind) {
    let index = lines
        .iter()
        .position(|line| line.number == Some(number) && line.text == text)
        .or_else(|| lines.iter().position(|line| line.number == Some(number)));

    if let Some(index) = index {
        lines[index].kind = kind;
    }
}

fn parse_patch_hunk_header(header: &str) -> Option<(u64, u64)> {
    let mut parts = header.split_whitespace();
    parts.next()?;
    let old_part = parts.next()?;
    let new_part = parts.next()?;
    Some((
        parse_patch_hunk_start(old_part)?,
        parse_patch_hunk_start(new_part)?,
    ))
}

fn parse_patch_hunk_start(part: &str) -> Option<u64> {
    part.trim_start_matches(['-', '+'])
        .split_once(',')
        .map_or(part.trim_start_matches(['-', '+']), |(start, _)| start)
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centers_around_highlighted_line() {
        let text = (1..=100)
            .map(|line| format!("line {line}"))
            .collect::<Vec<_>>()
            .join("\n");

        let lines = source_context_lines(&text, 75, None);

        assert_eq!(lines.first().and_then(|line| line.number), Some(65));
        assert_eq!(lines.last().and_then(|line| line.number), Some(85));
        assert!(lines.iter().any(|line| line.number == Some(75)));
    }

    #[test]
    fn clamps_to_file_start() {
        let lines = source_context_lines("one\ntwo\nthree", 2, None);

        assert_eq!(lines.first().and_then(|line| line.number), Some(1));
        assert_eq!(lines.last().and_then(|line| line.number), Some(3));
    }

    #[test]
    fn marks_added_lines_from_pr_patch() {
        let lines = source_context_lines(
            "test test test\nfor the gh-view\nto create a dead pr\nand have a discussions on it",
            2,
            Some(
                "@@ -0,0 +1,4 @@\n+test test test\n+for the gh-view\n+to create a dead pr\n+and have a discussions on it",
            ),
        );

        assert!(lines.iter().all(|line| line.kind == CodeLineKind::Added));
    }

    #[test]
    fn inserts_removed_lines_from_pr_patch() {
        let lines = source_context_lines(
            "kept\nnew\nafter",
            2,
            Some("@@ -1,3 +1,3 @@\n kept\n-old\n+new\n after"),
        );

        assert_eq!(lines.len(), 4);
        assert_eq!(lines[1].text, "old");
        assert_eq!(lines[1].kind, CodeLineKind::Removed);
        assert_eq!(lines[2].text, "new");
        assert_eq!(lines[2].kind, CodeLineKind::Added);
    }
}
