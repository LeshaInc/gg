use std::borrow::Cow;
use std::ops::Range;

use gg_assets::{Assets, Id};
use gg_math::Vec2;
use ttf_parser::GlyphId;
use unicode_linebreak::BreakOpportunity;

use crate::{
    Color, DrawGlyph, FontDb, FontFace, FontFamily, FontStyle, FontWeight, ShapedGlyph,
    ShapingCache,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Text<'a> {
    pub segments: Cow<'a, [TextSegment<'a>]>,
    pub props: TextProperties,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextProperties {
    pub line_height: f32,
    pub h_align: TextHAlign,
    pub v_align: TextVAlign,
    pub wrap: bool,
}

impl Default for TextProperties {
    fn default() -> Self {
        Self {
            line_height: 1.2,
            h_align: TextHAlign::Start,
            v_align: TextVAlign::Start,
            wrap: true,
        }
    }
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

#[derive(Clone, Debug, PartialEq)]
pub struct TextSegment<'a> {
    pub text: Cow<'a, str>,
    pub props: TextSegmentProperties,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextSegmentProperties {
    pub font_family: FontFamily,
    pub weight: FontWeight,
    pub style: FontStyle,
    pub size: f32,
    pub color: Color,
}

#[derive(Clone, Debug)]
pub struct ShapedText {
    props: TextProperties,
    segments: Vec<RawSegment>,
    glyphs: Vec<ShapedGlyph>,
}

#[derive(Debug, Default)]
pub struct TextLayouter {
    text: String,
    lines: Vec<Line>,
    segments: Vec<RawSegment>,
    scratch_segments: Vec<RawSegment>,
    glyphs: Vec<ShapedGlyph>,
    output_glyphs: Vec<DrawGlyph>,
    cache: ShapingCache,
}

#[derive(Clone, Debug)]
struct RawSegment {
    face: Option<Id<FontFace>>,
    range: Range<usize>,
    glyph_range: Range<usize>,
    tws_glyph_range: Range<usize>,
    props: TextSegmentProperties,
    linebreak: Option<BreakOpportunity>,
    flow_break: bool,
    width: f32,
    tws_width: f32,
    height: f32,
    ascender: f32,
}

impl RawSegment {
    fn new(props: TextSegmentProperties) -> RawSegment {
        RawSegment {
            face: None,
            range: 0..0,
            glyph_range: 0..0,
            tws_glyph_range: 0..0,
            props,
            linebreak: None,
            flow_break: false,
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

impl TextLayouter {
    pub fn new() -> TextLayouter {
        TextLayouter::default()
    }

    pub fn shape(&mut self, assets: &Assets, fonts: &FontDb, text: &Text) -> ShapedText {
        self.segments.clear();
        self.text.clear();
        self.append_text(text);

        find_linebreaks(&self.text, &mut self.segments, &mut self.scratch_segments);

        shape_segments(
            assets,
            fonts,
            &self.text,
            &mut self.segments,
            &mut self.glyphs,
            &mut self.cache,
        );

        measure_segments(assets, &text.props, &mut self.segments, &self.glyphs);

        ShapedText {
            props: text.props,
            segments: self.segments.clone(),
            glyphs: self.glyphs.clone(),
        }
    }

    pub fn measure(&mut self, text: &mut ShapedText, max_size: Vec2<f32>) -> Vec2<f32> {
        flow_segments(&mut text.segments, max_size.x, text.props.wrap);
        split_lines(&mut self.lines, &text.segments);
        measure_lines(&self.lines)
    }

    pub fn layout(
        &mut self,
        text: &mut ShapedText,
        max_size: Vec2<f32>,
    ) -> (Vec2<f32>, &[DrawGlyph]) {
        let size = self.measure(text, max_size);

        place_glyphs(
            &text.props,
            &self.lines,
            &text.segments,
            &text.glyphs,
            &mut self.output_glyphs,
            size,
            max_size,
        );

        (size, &self.output_glyphs)
    }

    fn append_text(&mut self, text: &Text) {
        for segment in text.segments.iter() {
            self.append_segment(segment);
        }
    }

    fn append_segment(&mut self, segment: &TextSegment) {
        if segment.text.is_empty() {
            return;
        }

        let start_idx = self.text.len();
        self.text.push_str(&segment.text);
        let range = start_idx..start_idx + segment.text.len();

        self.segments.push(RawSegment {
            range,
            ..RawSegment::new(segment.props.clone())
        });
    }
}

fn find_linebreaks(
    text: &str,
    segments: &mut Vec<RawSegment>,
    scratch_segments: &mut Vec<RawSegment>,
) {
    if segments.is_empty() {
        return;
    }

    scratch_segments.clear();

    let mut seg_i = 0;
    for (i, linebreak) in unicode_linebreak::linebreaks(text) {
        let segment = loop {
            let seg = &mut segments[seg_i];
            if seg.range.contains(&(i - 1)) {
                break seg;
            }

            scratch_segments.push(seg.clone());
            seg_i += 1;
        };

        if i < segment.range.end {
            scratch_segments.push(RawSegment {
                range: segment.range.start..i,
                linebreak: Some(linebreak),
                ..RawSegment::new(segment.props.clone())
            });

            segment.range.start = i;
        } else {
            segment.linebreak = Some(linebreak);
            scratch_segments.push(segment.clone());
            seg_i += 1;
        }
    }

    std::mem::swap(segments, scratch_segments);
}

fn shape_segments(
    assets: &Assets,
    fonts: &FontDb,
    text: &str,
    segments: &mut Vec<RawSegment>,
    glyphs: &mut Vec<ShapedGlyph>,
    cache: &mut ShapingCache,
) {
    glyphs.clear();

    let mut segment_i = 0;

    while segment_i < segments.len() {
        let mut segment = &mut segments[segment_i];
        segment_i += 1;

        let it = segment.props.font_family.names();
        let mut faces =
            it.flat_map(|name| fonts.find(name, segment.props.weight, segment.props.style));

        'outer: while let Some(face) = faces.next() {
            segment.face = Some(face.id());

            let face = &assets[face];
            let size = segment.props.size;

            let text = &text[segment.range.clone()];
            let text_no_ws = text.trim_end();
            let text_ws = &text[text_no_ws.len()..];

            let start_idx = glyphs.len();
            face.shape(cache, size, text_no_ws, glyphs);
            segment.glyph_range = start_idx..glyphs.len();

            let start_idx = glyphs.len();
            face.shape(cache, size, text_ws, glyphs);
            segment.tws_glyph_range = start_idx..glyphs.len();

            let mut missing_idx = usize::MAX;

            for glyph in &glyphs[segment.glyph_range.clone()] {
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

            let new_segment = RawSegment {
                range: split_idx..segment.range.end - text_ws.len(),
                linebreak: None,
                ..segment.clone()
            };

            let ws_segment = RawSegment {
                range: (segment.range.end - text_ws.len())..segment.range.end,
                linebreak: segment.linebreak.take(),
                ..segment.clone()
            };

            segment.range.end = split_idx;

            drop(faces);
            segments.insert(segment_i, new_segment);
            segments.insert(segment_i + 1, ws_segment);
            segment_i -= 1;
            break;
        }
    }
}

fn measure_segments(
    assets: &Assets,
    props: &TextProperties,
    segments: &mut [RawSegment],
    glyphs: &[ShapedGlyph],
) {
    for segment in segments {
        let face = match segment.face.map(|v| &assets[v]) {
            Some(v) => v,
            None => continue,
        };

        let metrics = face.line_metrics(segment.props.size);

        segment.height = props.line_height * segment.props.size;
        segment.ascender =
            metrics.ascender + (segment.height - metrics.ascender + metrics.descender) * 0.5;

        for glyph in &glyphs[segment.glyph_range.clone()] {
            segment.width += glyph.advance.x;
        }

        for glyph in &glyphs[segment.tws_glyph_range.clone()] {
            segment.tws_width += glyph.advance.x;
        }
    }
}

fn flow_segments(segments: &mut [RawSegment], max_width: f32, wrap: bool) {
    if segments.is_empty() {
        return;
    }

    for segment in segments.iter_mut() {
        segment.flow_break = segment.linebreak == Some(BreakOpportunity::Mandatory);
    }

    if !wrap {
        return;
    }

    let mut line_width = segments[0].width;
    let mut last_opportunity = 0;
    let mut i = 1;

    while i < segments.len() {
        if !segments[i - 1].flow_break {
            line_width += segments[i - 1].tws_width;
        }

        line_width += segments[i].width;

        if line_width > max_width
            && segments[last_opportunity].linebreak == Some(BreakOpportunity::Allowed)
            && !segments[last_opportunity].flow_break
        {
            line_width = 0.0;
            segments[last_opportunity].flow_break = true;
            i = last_opportunity + 1;
            continue;
        }

        match segments[i].linebreak {
            Some(BreakOpportunity::Allowed) => last_opportunity = i,
            Some(BreakOpportunity::Mandatory) => line_width = 0.0,
            _ => {}
        }

        i += 1
    }
}

fn split_lines(lines: &mut Vec<Line>, segments: &[RawSegment]) {
    lines.clear();

    let mut line = Line {
        range: 0..0,
        width: 0.0,
        height: 0.0,
        ascender: 0.0,
    };

    let mut i = 0;
    while i < segments.len() {
        let segment = &segments[i];
        i += 1;

        line.height = line.height.max(segment.height);
        line.ascender = line.ascender.max(segment.ascender);

        line.width += segment.width;

        if !segment.flow_break {
            line.width += segment.tws_width;
            continue;
        }

        line.range.end = i;
        lines.push(line.clone());
        line.range.start = i;

        line.width = 0.0;
        line.height = 0.0;
        line.ascender = 0.0;
    }
}

fn measure_lines(lines: &[Line]) -> Vec2<f32> {
    let mut size = Vec2::zero();

    for line in lines {
        size.x = line.width.max(size.x);
        size.y += line.height;
    }

    size
}

fn place_glyphs(
    props: &TextProperties,
    lines: &[Line],
    segments: &[RawSegment],
    glyphs: &[ShapedGlyph],
    output: &mut Vec<DrawGlyph>,
    size: Vec2<f32>,
    max_size: Vec2<f32>,
) {
    output.clear();

    let mut y = match props.v_align {
        TextVAlign::Start => 0.0,
        TextVAlign::Center => (max_size.y - size.y) * 0.5,
        TextVAlign::End => max_size.y - size.y,
    };

    for line in lines {
        let free = max_size.x - line.width;

        let x = match props.h_align {
            TextHAlign::Start => 0.0,
            TextHAlign::End => free,
            TextHAlign::Center => free * 0.5,
            TextHAlign::Justify => 0.0,
        };

        let mut min_width = size.x;
        let mut max_width = 0.0;
        let mut cur_width = 0.0;
        let mut num_spaced = 0.0;

        if props.h_align == TextHAlign::Justify {
            for segment in &segments[line.range.clone()] {
                cur_width += segment.width;
                if segment.linebreak.is_some() {
                    min_width = segment.width.min(cur_width);
                    max_width = segment.width.max(cur_width);
                    cur_width = 0.0;
                    num_spaced += 1.0;
                }
            }
        }

        let mut spacing = match props.h_align {
            TextHAlign::Justify => free / (num_spaced - 1.0),
            _ => 0.0,
        };

        let max_spacing = (min_width + max_width) * 0.5;

        if spacing > max_spacing {
            spacing = 0.0;
        }

        let mut cursor = Vec2::new(x, y);
        cursor.y += line.ascender;

        for segment in &segments[line.range.clone()] {
            let font = match segment.face {
                Some(v) => v,
                None => continue,
            };

            for glyph in &glyphs[segment.glyph_range.clone()] {
                output.push(DrawGlyph {
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
