use std::fmt::{self, Display};
use std::ops::Range;

use rowan::GreenNode;

use crate::syntax::{SyntaxNode, TextRange, TextSize};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SourceText {
    root: GreenNode,
    lines: Vec<TextRange>,
}

impl SourceText {
    pub fn new(root: GreenNode) -> SourceText {
        let text = SyntaxNode::new_root(root.clone()).text();

        let mut start = 0;
        let mut end = 0;
        let mut lines = Vec::new();

        text.for_each_chunk(|chunk| {
            if let Some(idx) = chunk.find('\n') {
                end += idx as u32;
                lines.push(TextRange::new(TextSize::from(start), TextSize::from(end)));
                start = end + 1;
            } else {
                end += chunk.len() as u32;
            }
        });

        SourceText { root, lines }
    }

    pub fn lines_in_range(&self, range: TextRange, extra: u32) -> Range<u32> {
        let last_line = self.lines.len().saturating_sub(1);

        let mut it = self.lines.iter();
        let start = it
            .position(|v| v.intersect(range).is_some())
            .unwrap_or(last_line);

        let mut it = self.lines.iter();
        let end = it
            .rposition(|v| v.intersect(range).is_some())
            .unwrap_or(last_line);

        let start = (start as u32).saturating_sub(extra);
        let end = (end as u32 + extra + 1).min(self.lines.len() as u32);
        start..end
    }

    pub fn line_text(&self, idx: u32) -> String {
        let line = self.lines[idx as usize];
        let text = SyntaxNode::new_root(self.root.clone()).text();
        let mut buf = String::with_capacity(line.len().into());
        text.slice(self.lines[idx as usize])
            .for_each_chunk(|chunk| buf.push_str(chunk));
        buf
    }

    pub fn line_range(&self, idx: u32) -> TextRange {
        self.lines[idx as usize]
    }

    pub fn range_to_line_col(&self, range: TextRange) -> LineColRange {
        let lines = self.lines_in_range(range, 0);

        let start_line_idx = lines.start;
        let start_line_range = self.lines[start_line_idx as usize];
        let mut start_line = self.line_text(start_line_idx);
        start_line.truncate((range.start() - start_line_range.start()).into());
        let start_col = start_line.chars().count();
        let start = LineColPos {
            line: start_line_idx + 1,
            col: start_col as u32 + 1,
        };

        let end_line_idx = lines.end - 1;
        let end_line_range = self.lines[end_line_idx as usize];
        let mut end_line = self.line_text(end_line_idx);
        end_line.truncate((range.end() - end_line_range.end()).into());
        let end_col = end_line.chars().count();
        let end = LineColPos {
            line: end_line_idx + 1,
            col: end_col as u32,
        };

        LineColRange { start, end }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Source {
    pub name: String,
    pub text: SourceText,
}

impl Source {
    pub fn new(name: String, text: SourceText) -> Source {
        Source { name, text }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineColPos {
    pub line: u32,
    pub col: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineColRange {
    pub start: LineColPos,
    pub end: LineColPos,
}

impl Display for LineColRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}-{}:{}",
            self.start.line, self.start.col, self.end.line, self.end.col
        )
    }
}
