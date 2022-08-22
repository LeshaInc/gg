use std::fmt::{self, Display};

use crate::syntax::Span;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
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
                .map(|(i, line)| {
                    let start = (line.as_ptr() as usize) - (text.as_ptr() as usize);
                    let end = start + line.len();
                    Line {
                        number: (i as u32) + 1,
                        span: Span::new(start as u32, end as u32),
                    }
                })
                .collect(),
            text,
        }
    }

    pub fn lines_in_span(&self, span: Span, extra: usize) -> &[Line] {
        let mut it = self.lines.iter();
        let start = it.position(|v| v.span.intersects(span)).unwrap_or(0);

        let mut it = self.lines.iter();
        let end = it.rposition(|v| v.span.intersects(span)).unwrap_or(0);

        &self.lines[start.saturating_sub(extra)..=(end + extra).min(self.lines.len() - 1)]
    }

    pub fn span_to_line_col(&self, span: Span) -> LineColSpan {
        let lines = self.lines_in_span(span, 0);

        let start_line = lines[0];
        let start_prefix = Span::new(lines[0].span.start, span.start).slice(&self.text);
        let start_col = start_prefix.chars().count() + 1;
        let start = LineColPos {
            line: start_line.number,
            col: start_col as u32,
        };

        let end_line = lines[lines.len() - 1];
        let end_prefix = Span::new(lines[lines.len() - 1].span.start, span.end).slice(&self.text);
        let end_col = end_prefix.chars().count();
        let end = LineColPos {
            line: end_line.number,
            col: end_col as u32,
        };

        LineColSpan { start, end }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Line {
    pub number: u32,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineColPos {
    pub line: u32,
    pub col: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineColSpan {
    pub start: LineColPos,
    pub end: LineColPos,
}

impl Display for LineColSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}-{}:{}",
            self.start.line, self.start.col, self.end.line, self.end.col
        )
    }
}
