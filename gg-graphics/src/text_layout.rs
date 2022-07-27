use std::ops::Range;

use gg_assets::{Assets, Id};
use gg_math::{Rect, Vec2};
use unicode_linebreak::BreakOpportunity;

use crate::{Color, DrawGlyph, Font, GraphicsEncoder, ShapedGlyph};

#[derive(Debug)]
pub struct TextLayouter {
    props: TextLayoutProperties,
    segments: Vec<Segment>,
    new_segments: Vec<Segment>,
    glyphs: Vec<ShapedGlyph>,
    text: String,
    size: Vec2<f32>,
    dirty: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct TextLayoutProperties {
    pub max_size: Vec2<f32>,
    pub line_height: f32,
}

impl Default for TextLayoutProperties {
    fn default() -> Self {
        Self {
            max_size: Vec2::splat(f32::INFINITY),
            line_height: 1.2,
        }
    }
}

#[derive(Clone, Debug)]
struct Segment {
    range: Range<usize>,
    glyph_range: Range<usize>,
    tws_glyph_range: Range<usize>,
    props: TextProperties,
    linebreak: Option<BreakOpportunity>,
    width: f32,
    tws_width: f32,
    height: f32,
    ascender: f32,
}

impl Segment {
    fn new(props: TextProperties) -> Segment {
        Segment {
            range: 0..0,
            glyph_range: 0..0,
            tws_glyph_range: 0..0,
            props,
            linebreak: None,
            width: 0.0,
            tws_width: 0.0,
            height: 0.0,
            ascender: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TextProperties {
    pub font: Id<Font>,
    pub size: f32,
    pub color: Color,
}

impl TextLayouter {
    pub fn new() -> TextLayouter {
        TextLayouter {
            props: TextLayoutProperties::default(),
            segments: Vec::new(),
            new_segments: Vec::new(),
            glyphs: Vec::new(),
            text: String::new(),
            size: Vec2::zero(),
            dirty: false,
        }
    }

    pub fn reset(&mut self) {
        self.segments.clear();
        self.new_segments.clear();
        self.glyphs.clear();
        self.text.clear();
        self.dirty = true;
    }

    pub fn append(&mut self, props: TextProperties, text: &str) {
        if text.is_empty() {
            return;
        }

        let start_idx = self.text.len();
        self.text.push_str(text);
        let range = start_idx..start_idx + text.len();

        self.segments.push(Segment {
            range,
            ..Segment::new(props)
        });
    }

    pub fn set_props(&mut self, props: &TextLayoutProperties) {
        self.props = *props;
        self.dirty = true;
    }

    pub fn layout(&mut self, assets: &Assets) -> Vec2<f32> {
        if !self.dirty {
            return self.size;
        }

        self.split_linebreaks();
        self.shape(assets);
        self.measure(assets);
        self.flow();

        self.size
    }

    pub fn draw(&mut self, assets: &Assets, encoder: &mut GraphicsEncoder, bounds: Rect<f32>) {
        self.layout(assets);

        let mut pos = bounds.min;

        for segment in &self.segments {
            let mut cursor = pos;
            cursor.y += segment.ascender;

            for glyph in &self.glyphs[segment.glyph_range.clone()] {
                encoder.glyph(DrawGlyph {
                    font: segment.props.font,
                    glyph: glyph.glyph,
                    size: segment.props.size,
                    pos: cursor + glyph.offset,
                    color: segment.props.color,
                });

                cursor.x += glyph.advance.x;
            }

            pos.x += segment.width;

            if segment.linebreak == Some(BreakOpportunity::Mandatory) {
                pos.x = bounds.min.x;
                pos.y += segment.height;
            } else {
                pos.x += segment.tws_width;
            }
        }
    }

    fn split_linebreaks(&mut self) {
        if self.segments.is_empty() {
            return;
        }

        self.new_segments.clear();

        let mut seg_i = 0;
        for (i, linebreak) in unicode_linebreak::linebreaks(&self.text) {
            let segment = loop {
                let seg = &mut self.segments[seg_i];
                if seg.range.contains(&(i - 1)) {
                    break seg;
                }

                self.new_segments.push(seg.clone());
                seg_i += 1;
            };

            if i < segment.range.end {
                self.new_segments.push(Segment {
                    range: segment.range.start..i,
                    linebreak: Some(linebreak),
                    ..Segment::new(segment.props)
                });

                segment.range.start = i;
            } else {
                segment.linebreak = (i != self.text.len()).then_some(linebreak);
                self.new_segments.push(segment.clone());
                seg_i += 1;
            }
        }

        std::mem::swap(&mut self.segments, &mut self.new_segments);
    }

    fn shape(&mut self, assets: &Assets) {
        self.glyphs.clear();

        for segment in &mut self.segments {
            let font = &assets[segment.props.font];
            let text = &self.text[segment.range.clone()];

            let text_nows = text.trim_end();
            let text_ws = &text[text_nows.len()..];

            let start_idx = self.glyphs.len();
            font.shape2(segment.props.size, text_nows, &mut self.glyphs);
            segment.glyph_range = start_idx..self.glyphs.len();

            let start_idx = self.glyphs.len();
            font.shape2(segment.props.size, text_ws, &mut self.glyphs);
            segment.tws_glyph_range = start_idx..self.glyphs.len();
        }
    }

    fn measure(&mut self, assets: &Assets) {
        for segment in &mut self.segments {
            let font = &assets[segment.props.font];
            let metrics = font.line_metrics(segment.props.size);

            segment.height = self.props.line_height * segment.props.size;
            segment.ascender =
                metrics.ascender + (segment.height - metrics.ascender + metrics.descender) * 0.5;

            for i in segment.glyph_range.clone() {
                let glyph = &self.glyphs[i];
                segment.width += glyph.advance.x;
            }

            for i in segment.tws_glyph_range.clone() {
                let glyph = &self.glyphs[i];
                segment.tws_width += glyph.advance.x;
            }
        }
    }

    fn flow(&mut self) {
        if self.segments.is_empty() {
            return;
        }

        let max_width = self.props.max_size.x;

        let mut line_width = self.segments[0].width;
        let mut line_height = self.segments[0].height;

        self.size = Vec2::zero();

        for i in 1..self.segments.len() {
            let (before, after) = self.segments.split_at_mut(i);
            let prev_segment = &mut before[i - 1];
            let segment = &after[0];

            line_width += prev_segment.tws_width + segment.width;

            if line_width > max_width {
                self.size.x = self.size.x.max(line_width);
                self.size.y += line_height;

                line_width = segment.width;
                line_height = segment.height;
                // TODO: this is wrong
                prev_segment.linebreak = Some(BreakOpportunity::Mandatory);
            } else {
                line_height = line_height.max(segment.height);
            }
        }

        // TODO: update height & ascender

        self.size.x = self.size.x.max(line_width);
        self.size.y += line_height;
    }
}
