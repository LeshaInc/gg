use std::fmt::{self, Display};
use std::ops::Range;
use std::sync::Arc;

use unicode_width::UnicodeWidthStr;
use yansi::{Color, Paint};

use crate::syntax::TextRange;
use crate::Source;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    pub fn name(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Severity::Info => Color::Blue,
            Severity::Warning => Color::Yellow,
            Severity::Error => Color::Red,
        }
    }
}

impl Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Paint::new(self.name()).fg(self.color()).bold().fmt(f)
    }
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub components: Vec<Component>,
}

impl Diagnostic {
    pub fn new(severity: Severity, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            severity,
            message: message.into(),
            components: Vec::new(),
        }
    }

    pub fn add_help(&mut self, help: impl Into<String>) {
        self.components.push(Component::Help(help.into()));
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Diagnostic {
        self.add_help(help);
        self
    }

    pub fn add_source(&mut self, source: SourceComponent) {
        self.components.push(Component::Source(source));
    }

    pub fn with_source(mut self, source: SourceComponent) -> Diagnostic {
        self.add_source(source);
        self
    }
}

impl Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}: {}", self.severity, Paint::new(&self.message).bold())?;

        for component in &self.components {
            write!(f, "{}", component)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum Component {
    Help(String),
    Source(SourceComponent),
}

impl Display for Component {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Component::Help(v) => {
                writeln!(
                    f,
                    "   {}{}{}",
                    Paint::cyan("help").bold(),
                    Paint::new(": ").bold(),
                    Paint::new(v).bold()
                )
            }
            Component::Source(v) => v.fmt(f),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SourceComponent {
    pub source: Arc<Source>,
    pub labels: Vec<Label>,
}

impl SourceComponent {
    pub fn new(source: Arc<Source>) -> SourceComponent {
        SourceComponent {
            source,
            labels: Vec::new(),
        }
    }

    pub fn with_label(
        mut self,
        severity: Severity,
        range: TextRange,
        message: impl Into<String>,
    ) -> SourceComponent {
        self.labels.push(Label {
            severity,
            range,
            message: message.into(),
        });

        self
    }
}

#[derive(Clone, Debug)]
pub struct Label {
    pub severity: Severity,
    pub range: TextRange,
    pub message: String,
}

impl Display for SourceComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let max_range = max_range(self.labels.iter().map(|l| l.range));

        let lines = self.source.text.lines_in_range(max_range, 1);
        if lines.is_empty() {
            return Ok(());
        }

        let mut grid = HlGrid::new(&self.source, lines);
        for label in &self.labels {
            grid.add_label(label);
        }

        grid.place_labels();
        grid.place_connectors();
        grid.fmt(f)
    }
}

#[derive(Clone, Debug)]
struct HlGrid {
    name: String,
    lines: Vec<HlLine>,
    connectors: Vec<Connector>,
    cells: Vec<Vec<(char, Color)>>,
}

#[derive(Clone, Debug)]
struct Connector {
    start_i: usize,
    start_y: usize,
    end_i: usize,
    end_y: usize,
    color: Color,
}

impl HlGrid {
    fn new(source: &Source, lines: Range<u32>) -> HlGrid {
        let lines = lines
            .map(|idx| {
                let text = source.text.line_text(idx);
                let range = source.text.line_range(idx);
                HlLine {
                    number: idx + 1,
                    text_width: text.width(),
                    text,
                    range,
                    cells: Vec::new(),
                    labels: Vec::new(),
                }
            })
            .collect();

        HlGrid {
            name: source.name.clone(),
            lines,
            connectors: Vec::new(),
            cells: Vec::new(),
        }
    }

    fn add_label(&mut self, label: &Label) {
        let mut it = self.lines.iter_mut().enumerate();
        if let Some((start_i, start_line)) =
            it.find(|(_, line)| line.range.intersect(label.range).is_some())
        {
            start_line.add_label(label);
            let start_y = start_line.height() - 1;

            let mut it = self.lines.iter_mut().enumerate();
            if let Some((end_i, end_line)) =
                it.rfind(|(_, line)| line.range.intersect(label.range).is_some())
            {
                if start_i == end_i {
                    return;
                }

                end_line.add_label(label);
                let end_y = end_line.height() - 1;

                self.connectors.push(Connector {
                    start_i,
                    start_y,
                    end_i,
                    end_y,
                    color: label.severity.color(),
                });
            }
        }
    }

    fn place_labels(&mut self) {
        for line in &mut self.lines {
            line.place_labels();
        }
    }

    fn place_connectors(&mut self) {
        let mut total_height = 0;
        for line in &mut self.lines {
            total_height += line.height() + 1;
        }

        self.connectors.sort_by_key(|v| v.start_i);

        let mut height = 1;
        let mut line_i = 0;
        for connector in &mut self.connectors {
            while line_i < connector.start_i {
                height += self.lines[line_i].height() + 1;
                line_i += 1;
            }

            connector.start_y += height;
        }

        self.connectors.sort_by_key(|v| v.end_i);

        let mut height = 1;
        let mut line_i = 0;
        for connector in &mut self.connectors {
            while line_i < connector.end_i {
                height += self.lines[line_i].height() + 1;
                line_i += 1;
            }

            connector.end_y += height;
        }

        self.connectors.sort_by_key(|v| v.end_y - v.start_y);

        for connector in &mut self.connectors {
            let color = connector.color;

            let pos = connector.start_y;
            let len = connector.end_y - connector.start_y + 1;

            let mut x = 0;
            loop {
                if x >= self.cells.len() {
                    self.cells.push(vec![(' ', Color::Unset); total_height]);
                }

                let cells = &mut self.cells[x][pos..pos + len];

                if cells.iter().any(|v| v.0 == '│') {
                    x += 1;
                    continue;
                }

                for cell in cells.iter_mut() {
                    *cell = ('│', color);
                }

                cells[0].0 = '╭';
                cells[cells.len() - 1].0 = '╰';

                for y in [pos, pos + len - 1] {
                    for x in 0..x {
                        let cell = &mut self.cells[x][y];
                        *cell = match cell.0 {
                            '│' => ('┊', cell.1),
                            _ => ('─', color),
                        }
                    }
                }

                break;
            }
        }
    }
}

impl Display for HlGrid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut y = 0;

        let style = Color::Cyan.style().bold();
        let width = decimal_width(self.lines[self.lines.len() - 1].number);

        writeln!(
            f,
            "{}{} {}",
            " ".repeat(width + 2),
            style.paint("╭──"),
            Color::Cyan.paint(&self.name).bold().underline()
        )?;

        for (line_i, line) in self.lines.iter().enumerate() {
            let number = style.paint(line.number);
            write!(f, " {0:>1$} {2} ", number, width, style.paint("│"))?;

            for col in self.cells.iter().rev() {
                let (char, color) = col[y];
                write!(f, "{}", color.paint(char).bold())?;
            }

            writeln!(f, "{}", line.text)?;
            y += 1;

            for row in &line.cells {
                let last = line_i == self.lines.len() - 1;
                let c = if last { " " } else { "┆" };
                write!(f, " {0:>1$} {2} ", "", width, style.paint(c))?;

                for col in self.cells.iter().rev() {
                    let (char, color) = col[y];
                    write!(f, "{}", color.paint(char).bold())?;
                }

                for (char, color) in row {
                    write!(f, "{}", color.paint(char).bold().italic())?;
                }
                writeln!(f)?;
                y += 1;
            }
        }

        // writeln!(f, "{}{} ", " ".repeat(width + 2), style.paint("╰──"))?;

        Ok(())
    }
}

#[derive(Clone, Debug)]
struct HlLine {
    number: u32,
    text: String,
    text_width: usize,
    range: TextRange,
    cells: Vec<Vec<(char, Color)>>,
    labels: Vec<(usize, usize, Color, String)>,
}

impl HlLine {
    fn width(&self) -> usize {
        if self.cells.is_empty() {
            self.text_width + 2
        } else {
            self.cells[0].len()
        }
    }

    fn height(&self) -> usize {
        self.cells.len()
    }

    fn extend_down(&mut self, height: usize) {
        while self.height() < height {
            let cell = (' ', Color::Unset);
            self.cells.push(vec![cell; self.width()]);
        }
    }

    fn extend_right(&mut self, width: usize) {
        while self.width() < width {
            for row in &mut self.cells {
                row.push((' ', Color::Unset));
            }
        }
    }

    fn add_label(&mut self, label: &Label) {
        self.extend_down(1);

        let range = label.range;
        let color = label.severity.color();

        let start = usize::from(range.start()).saturating_sub(usize::from(self.range.start()));

        let extend_right = usize::from(range.end()) > usize::from(self.range.end()) + 1;
        let end = usize::from(range.end()).min(usize::from(self.range.end()))
            - usize::from(self.range.start());

        let pos = self.text[..start].width();
        let len = self.text[start..end].width().max(1);

        let mut y = 0;
        let cells = loop {
            self.extend_down(y + 1);

            let cells = &mut self.cells[y][pos..pos + len + 1];
            if cells.iter().any(|v| v.0 != ' ') {
                y += 1;
                continue;
            }

            break cells;
        };

        for cell in cells.iter_mut().take(len) {
            *cell = ('━', color);
        }

        if extend_right {
            cells[cells.len() - 1] = ('╮', color);

            let ny = self.height();
            self.extend_down(ny + 1);

            for cell in &mut self.cells[ny][..self.text_width] {
                *cell = ('─', color);
            }

            self.cells[ny][self.text_width] = ('╯', color);
            self.vline(color, self.text_width, y + 1, ny - 1);
            return;
        }

        let mut label_pos = pos + 1.min(len - 1);
        if self.labels.iter().any(|&(pos, ..)| pos == label_pos) {
            label_pos = pos;
        }

        while label_pos < pos + len && self.labels.iter().any(|&(pos, ..)| pos == label_pos) {
            label_pos += 1;
        }

        if label.message.trim().is_empty() {
            return;
        }

        cells[label_pos - pos] = ('┯', color);

        self.labels
            .push((label_pos, y, color, label.message.clone()))
    }

    fn vline(&mut self, color: Color, x: usize, y0: usize, y1: usize) {
        for y in y0..=y1 {
            let cell = &mut self.cells[y][x];
            cell.0 = match cell.0 {
                '─' => '┊',
                _ => '│',
            };
            cell.1 = color;
        }
    }

    fn place_labels(&mut self) {
        let mut labels = std::mem::take(&mut self.labels);
        labels.sort_by_key(|v| v.0);

        let mut sy = self.height();

        for &(x, y, color, ref label) in labels.iter().rev() {
            self.extend_down(sy + 1);
            self.extend_right(x + label.chars().count() + 4);

            self.vline(color, x, y + 1, sy - 1);
            self.cells[sy][x] = ('╰', color);
            self.cells[sy][x + 1] = ('─', color);
            self.cells[sy][x + 2] = ('─', color);

            for (i, c) in label.chars().enumerate() {
                self.cells[sy][x + 4 + i] = (c, color);
            }

            sy += 1;
        }
    }
}

fn max_range(ranges: impl Iterator<Item = TextRange>) -> TextRange {
    ranges.reduce(TextRange::cover).unwrap_or_default()
}

fn decimal_width(v: u32) -> usize {
    if v == 0 {
        return 0;
    }

    ((v as f32).log10() + 1.0).trunc() as usize
}
