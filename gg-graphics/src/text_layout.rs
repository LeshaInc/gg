use std::ops::Range;

use gg_assets::{Assets, Id};
use gg_math::{Rect, Vec2};
use ttf_parser::GlyphId;
use unicode_linebreak::BreakOpportunity;

use crate::{
    Color, DrawGlyph, FontDb, FontFace, FontFamily, FontStyle, FontWeight, GraphicsEncoder,
    ShapedGlyph,
};

#[derive(Debug)]
pub struct TextLayouter {
    props: TextLayoutProperties,
    segments: Vec<Segment>,
    new_segments: Vec<Segment>,
    glyphs: Vec<ShapedGlyph>,
    lines: Vec<Line>,
    text: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextHAlign {
    Start,
    Center,
    Justify,
    End,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextVAlign {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug)]
pub struct TextLayoutProperties {
    pub line_height: f32,
    pub h_align: TextHAlign,
    pub v_align: TextVAlign,
}

impl Default for TextLayoutProperties {
    fn default() -> Self {
        Self {
            line_height: 1.2,
            h_align: TextHAlign::Start,
            v_align: TextVAlign::Start,
        }
    }
}

#[derive(Clone, Debug)]
struct Segment {
    face: Option<Id<FontFace>>,
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
            face: None,
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

#[derive(Clone, Debug)]
struct Line {
    range: Range<usize>,
    width: f32,
    height: f32,
    ascender: f32,
}

#[derive(Clone, Debug)]
pub struct TextProperties {
    pub font_family: FontFamily,
    pub weight: FontWeight,
    pub style: FontStyle,
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
            lines: Vec::new(),
            text: String::new(),
        }
    }

    pub fn reset(&mut self) {
        self.segments.clear();
        self.new_segments.clear();
        self.glyphs.clear();
        self.lines.clear();
        self.text.clear();
    }

    pub fn append(&mut self, props: &TextProperties, text: &str) {
        if text.is_empty() {
            return;
        }

        let start_idx = self.text.len();
        self.text.push_str(text);
        let range = start_idx..start_idx + text.len();

        self.segments.push(Segment {
            range,
            ..Segment::new(props.clone())
        });
    }

    pub fn set_props(&mut self, props: &TextLayoutProperties) {
        self.props = *props;
    }

    pub fn layout(
        &mut self,
        assets: &Assets,
        fonts: &FontDb,
        max_size: Vec2<f32>,
        buf: &mut Vec<DrawGlyph>,
    ) {
        self.find_linebreaks();
        self.shape_segments(assets, fonts);
        self.measure_segments(assets);
        self.flow_segments(max_size.x);
        self.split_lines();
        let size = self.measure_lines();
        self.place_glyphs(size, max_size, buf);
    }

    pub fn draw(
        &mut self,
        assets: &Assets,
        fonts: &FontDb,
        encoder: &mut GraphicsEncoder,
        bounds: Rect<f32>,
    ) {
        let mut buf = Vec::new();
        self.layout(assets, fonts, bounds.extents(), &mut buf);
        for mut glyph in buf {
            glyph.pos += bounds.min;
            encoder.glyph(glyph);
        }
    }

    fn find_linebreaks(&mut self) {
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
                    ..Segment::new(segment.props.clone())
                });

                segment.range.start = i;
            } else {
                segment.linebreak = Some(linebreak);
                self.new_segments.push(segment.clone());
                seg_i += 1;
            }
        }

        std::mem::swap(&mut self.segments, &mut self.new_segments);
    }

    fn shape_segments(&mut self, assets: &Assets, fonts: &FontDb) {
        self.glyphs.clear();

        let mut segment_i = 0;

        while segment_i < self.segments.len() {
            let mut segment = &mut self.segments[segment_i];
            segment_i += 1;

            let it = segment.props.font_family.names();
            let mut faces =
                it.flat_map(|name| fonts.find(name, segment.props.weight, segment.props.style));

            'outer: while let Some(face) = faces.next() {
                segment.face = Some(face.id());

                let face = &assets[face];

                let text = &self.text[segment.range.clone()];
                let text_no_ws = text.trim_end();
                let text_ws = &text[text_no_ws.len()..];

                let start_idx = self.glyphs.len();
                face.shape(segment.props.size, text_no_ws, &mut self.glyphs);
                segment.glyph_range = start_idx..self.glyphs.len();

                let start_idx = self.glyphs.len();
                face.shape(segment.props.size, text_ws, &mut self.glyphs);
                segment.tws_glyph_range = start_idx..self.glyphs.len();

                let mut missing_idx = usize::MAX;

                for glyph in &self.glyphs[segment.glyph_range.clone()] {
                    if glyph.glyph == GlyphId(0) {
                        if glyph.cluster == 0 && text_ws.is_empty() {
                            continue 'outer;
                        }

                        missing_idx = glyph.cluster as usize;
                        break;
                    }
                }

                if missing_idx == usize::MAX {
                    break;
                }

                let split_idx = segment.range.start + missing_idx;

                let new_segment = Segment {
                    range: split_idx..segment.range.end - text_ws.len(),
                    linebreak: None,
                    ..segment.clone()
                };

                let ws_segment = Segment {
                    range: (segment.range.end - text_ws.len())..segment.range.end,
                    linebreak: segment.linebreak.take(),
                    ..segment.clone()
                };

                segment.range.end = split_idx;

                drop(faces);
                self.segments.insert(segment_i, new_segment);
                self.segments.insert(segment_i + 1, ws_segment);
                segment_i -= 1;
                break;
            }
        }
    }

    fn measure_segments(&mut self, assets: &Assets) {
        for segment in &mut self.segments {
            let face = match segment.face.map(|v| &assets[v]) {
                Some(v) => v,
                None => continue,
            };

            let metrics = face.line_metrics(segment.props.size);

            segment.height = self.props.line_height * segment.props.size;
            segment.ascender =
                metrics.ascender + (segment.height - metrics.ascender + metrics.descender) * 0.5;

            for glyph in &self.glyphs[segment.glyph_range.clone()] {
                segment.width += glyph.advance.x;
            }

            for glyph in &self.glyphs[segment.tws_glyph_range.clone()] {
                segment.tws_width += glyph.advance.x;
            }
        }
    }

    fn flow_segments(&mut self, max_width: f32) {
        if self.segments.is_empty() {
            return;
        }

        let mut line_width = self.segments[0].width;
        let mut last_opportunity = 0;
        let mut i = 1;

        while i < self.segments.len() {
            let prev_segment = &self.segments[i - 1];
            let segment = &self.segments[i];
            i += 1;

            line_width += prev_segment.tws_width + segment.width;

            if line_width > max_width
                && self.segments[last_opportunity].linebreak == Some(BreakOpportunity::Allowed)
            {
                line_width = 0.0;
                self.segments[last_opportunity].linebreak = Some(BreakOpportunity::Mandatory);
                i = last_opportunity + 1;
                continue;
            }

            match segment.linebreak {
                Some(BreakOpportunity::Allowed) => last_opportunity = i - 1,
                Some(BreakOpportunity::Mandatory) => line_width = 0.0,
                _ => {}
            }
        }
    }

    fn measure_lines(&self) -> Vec2<f32> {
        let mut size = Vec2::zero();

        for line in &self.lines {
            size.x = line.width.max(size.x);
            size.y += line.height;
        }

        size
    }

    fn split_lines(&mut self) {
        self.lines.clear();

        let mut line = Line {
            range: 0..0,
            width: 0.0,
            height: 0.0,
            ascender: 0.0,
        };

        let mut i = 0;
        while i < self.segments.len() {
            let segment = &self.segments[i];
            i += 1;

            line.height = line.height.max(segment.height);
            line.ascender = line.ascender.max(segment.ascender);

            line.width += segment.width;

            if segment.linebreak != Some(BreakOpportunity::Mandatory) {
                line.width += segment.tws_width;
                continue;
            }

            line.range.end = i;
            self.lines.push(line.clone());
            line.range.start = i;

            line.width = 0.0;
            line.height = 0.0;
            line.ascender = 0.0;
        }
    }

    fn place_glyphs(&mut self, size: Vec2<f32>, max_size: Vec2<f32>, buf: &mut Vec<DrawGlyph>) {
        let mut y = match self.props.v_align {
            TextVAlign::Start => 0.0,
            TextVAlign::Center => (max_size.y - size.y) * 0.5,
            TextVAlign::End => max_size.y - size.y,
        };

        for line in &self.lines {
            let free = max_size.x - line.width;

            let x = match self.props.h_align {
                TextHAlign::Start => 0.0,
                TextHAlign::End => free,
                TextHAlign::Center => free * 0.5,
                TextHAlign::Justify => 0.0,
            };

            let mut min_width = size.x;
            let mut max_width = 0.0;
            let mut cur_width = 0.0;
            let mut num_spaced = 0.0;

            if self.props.h_align == TextHAlign::Justify {
                for segment in &self.segments[line.range.clone()] {
                    cur_width += segment.width;
                    if segment.linebreak.is_some() {
                        min_width = segment.width.min(cur_width);
                        max_width = segment.width.max(cur_width);
                        cur_width = 0.0;
                        num_spaced += 1.0;
                    }
                }
            }

            let mut spacing = match self.props.h_align {
                TextHAlign::Justify => free / (num_spaced - 1.0),
                _ => 0.0,
            };

            let max_spacing = (min_width + max_width) * 0.5;

            if spacing > max_spacing {
                spacing = 0.0;
            }

            let mut cursor = Vec2::new(x, y);
            cursor.y += line.ascender;

            for segment in &self.segments[line.range.clone()] {
                let font = match segment.face {
                    Some(v) => v,
                    None => continue,
                };

                for glyph in &self.glyphs[segment.glyph_range.clone()] {
                    buf.push(DrawGlyph {
                        font,
                        glyph: glyph.glyph,
                        size: segment.props.size,
                        pos: cursor + glyph.offset,
                        color: segment.props.color,
                    });

                    cursor.x += glyph.advance.x;
                }

                cursor.x += segment.tws_width;

                if segment.linebreak.is_some() {
                    cursor.x += spacing;
                }
            }

            y += line.height;
        }
    }
}
