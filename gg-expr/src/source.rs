use std::fmt::{self, Display};

use crate::syntax::{TextRange, TextSize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct Source {
    pub name: String,
    pub text: String,
    pub lines: Vec<Line>,
}

impl Source {
    pub fn new(name: String, text: String) -> Source {
        Source {
            name,
            lines: text
                .lines()
                .enumerate()
                .map(|(i, line)| -> Line {
                    let start = (line.as_ptr() as usize) - (text.as_ptr() as usize);
                    let end = start + line.len();
                    Line {
                        number: (i as u32) + 1,
                        range: TextRange::new(
                            TextSize::from(start as u32),
                            TextSize::from(end as u32),
                        ),
                    }
                })
                .collect(),
            text,
        }
    }

    pub fn lines_in_range(&self, range: TextRange, extra: usize) -> &[Line] {
        let mut it = self.lines.iter();
        let start = it
            .position(|v| v.range.intersect(range).is_some())
            .unwrap_or(0);

        let mut it = self.lines.iter();
        let end = it
            .rposition(|v| v.range.intersect(range).is_some())
            .unwrap_or(0);

        &self.lines[start.saturating_sub(extra)..=(end + extra).min(self.lines.len() - 1)]
    }

    pub fn range_to_line_col(&self, range: TextRange) -> LineColRange {
        let lines = self.lines_in_range(range, 0);

        let start_line = lines[0];
        let start_prefix = &self.text[TextRange::new(lines[0].range.start(), range.start())];
        let start_col = start_prefix.chars().count() + 1;
        let start = LineColPos {
            line: start_line.number,
            col: start_col as u32,
        };

        let end_line = lines[lines.len() - 1];
        let end_prefix =
            &self.text[TextRange::new(lines[lines.len() - 1].range.start(), range.end())];
        let end_col = end_prefix.chars().count();
        let end = LineColPos {
            line: end_line.number,
            col: end_col as u32,
        };

        LineColRange { start, end }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Line {
    pub number: u32,
    pub range: TextRange,
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
