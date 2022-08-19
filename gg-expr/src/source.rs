use crate::syntax::Span;

#[derive(Clone, Debug)]
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
}

#[derive(Clone, Debug)]
pub struct Line {
    pub number: u32,
    pub span: Span,
}
