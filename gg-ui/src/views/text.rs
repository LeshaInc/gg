use std::marker::PhantomData;

use gg_graphics::{
    Color, DrawGlyph, FontFamily, FontStyle, FontWeight, TextHAlign, TextLayoutProperties,
    TextProperties, TextVAlign,
};
use gg_math::{Rect, Vec2};

use crate::{DrawCtx, LayoutCtx, View};

pub fn text<D>(text: impl Into<String>) -> Text<D> {
    Text {
        phantom: PhantomData,
        text: text.into(),
        glyphs: Vec::new(),
    }
}

pub struct Text<D> {
    phantom: PhantomData<D>,
    text: String,
    glyphs: Vec<DrawGlyph>,
}

impl<D> View<D> for Text<D> {
    fn update(&mut self, old: &mut Self) -> bool
    where
        Self: Sized,
    {
        if self.text == old.text {
            std::mem::swap(&mut self.glyphs, &mut old.glyphs);
            false
        } else {
            true
        }
    }

    fn layout(&mut self, ctx: LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        ctx.text_layouter.set_props(&TextLayoutProperties {
            line_height: 1.2,
            h_align: TextHAlign::Start,
            v_align: TextVAlign::Start,
        });

        let props = TextProperties {
            font_family: FontFamily::new("Open Sans")
                .push("Noto Color Emoji")
                .push("Noto Sans")
                .push("Noto Sans JP"),
            weight: FontWeight::Normal,
            style: FontStyle::Normal,
            size: 20.0,
            color: Color::WHITE,
        };

        ctx.text_layouter.reset();
        ctx.text_layouter.append(&props, &self.text);
        let (size, glyphs) = ctx.text_layouter.layout(ctx.assets, ctx.fonts, size);

        self.glyphs.clear();
        self.glyphs.extend_from_slice(glyphs);

        size
    }

    fn draw(&mut self, ctx: DrawCtx, bounds: Rect<f32>) {
        for glyph in &self.glyphs {
            let mut glyph = *glyph;
            glyph.pos += bounds.min;
            ctx.encoder.glyph(glyph);
        }
    }
}
