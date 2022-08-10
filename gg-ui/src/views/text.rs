use std::borrow::Cow;
use std::marker::PhantomData;

use gg_graphics::{
    Color, FontFamily, FontStyle, FontWeight, ShapedText, Text, TextProperties, TextSegment,
    TextSegmentProperties,
};
use gg_math::Vec2;

use crate::{Bounds, DrawCtx, LayoutCtx, View};

pub fn text<D>(text: impl Into<String>) -> TextView<D> {
    TextView {
        phantom: PhantomData,
        text: text.into(),
        props: TextProperties::default(),
        shaped_text: None,
    }
}

pub struct TextView<D> {
    phantom: PhantomData<fn(D)>,
    text: String,
    props: TextProperties,
    shaped_text: Option<ShapedText>,
}

impl<D> TextView<D> {
    pub fn wrap(mut self, v: bool) -> Self {
        self.props.wrap = v;
        self
    }
}

impl<D> View<D> for TextView<D> {
    fn update(&mut self, old: &mut Self) -> bool
    where
        Self: Sized,
    {
        if self.text == old.text {
            self.shaped_text = old.shaped_text.take();
            false
        } else {
            true
        }
    }

    fn layout(&mut self, ctx: LayoutCtx, size: Vec2<f32>) -> Vec2<f32> {
        let shaped_text = self.shaped_text.get_or_insert_with(|| {
            let segments = [TextSegment {
                text: Cow::Borrowed(&self.text),
                props: TextSegmentProperties {
                    font_family: FontFamily::new("Open Sans")
                        .push("Noto Color Emoji")
                        .push("Noto Sans")
                        .push("Noto Sans JP"),
                    weight: FontWeight::Normal,
                    style: FontStyle::Normal,
                    size: 20.0,
                    color: Color::WHITE,
                },
            }];

            let text = Text {
                segments: Cow::Borrowed(&segments),
                props: self.props,
            };

            ctx.text_layouter.shape(ctx.assets, ctx.fonts, &text)
        });

        ctx.text_layouter.measure(shaped_text, size).fmax(size)
    }

    fn draw(&mut self, ctx: DrawCtx, bounds: Bounds) {
        if let Some(text) = &mut self.shaped_text {
            let (_size, glyphs) = ctx.text_layouter.layout(text, bounds.rect.size());

            for glyph in glyphs {
                let mut glyph = *glyph;
                glyph.pos += bounds.rect.min;
                ctx.encoder.glyph(glyph);
            }
        }
    }
}
