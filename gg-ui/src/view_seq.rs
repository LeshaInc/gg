use gg_math::{Rect, Vec2};

use crate::{DrawCtx, Event, HandleCtx, LayoutCtx, LayoutHints, View};

pub trait ViewSeq<D> {
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn update(&mut self, old: &mut Self, index: usize) -> bool;

    fn pre_layout(&mut self, ctx: LayoutCtx, idx: usize) -> LayoutHints;

    fn layout(&mut self, ctx: LayoutCtx, size: Vec2<f32>, idx: usize) -> Vec2<f32>;

    fn draw(&mut self, ctx: DrawCtx, bounds: Rect<f32>, idx: usize);

    fn handle(&mut self, ctx: HandleCtx<D>, bounds: Rect<f32>, event: Event, idx: usize);
}

pub trait MetaSeq<T> {
    type MetaSeq: AsRef<[T]> + AsMut<[T]>;

    fn new_meta_seq<F>(ctor: F) -> Self::MetaSeq
    where
        F: FnMut() -> T;
}

macro_rules! impl_tuple {
    ($len:literal, $( $i:tt => $V:ident ),+) => {
        impl<D, $( $V, )+> ViewSeq<D> for ($( $V, )+)
        where
            $( $V: View<D>, )+
        {
            fn len(&self) -> usize {
                $len
            }

            fn update(&mut self, old: &mut Self, idx: usize) -> bool {
                match idx {
                    $( $i => self.$i.update(&mut old.$i), )+
                    _ => panic!("index out of bounds"),
                }
            }

            fn pre_layout(&mut self, ctx: LayoutCtx, idx: usize) -> LayoutHints {
                match idx {
                    $( $i => self.$i.pre_layout(ctx), )+
                    _ => panic!("index out of bounds"),
                }
            }

            fn layout(&mut self, ctx: LayoutCtx, size: Vec2<f32>, idx: usize) -> Vec2<f32> {
                match idx {
                    $( $i => self.$i.layout(ctx, size), )+
                    _ => panic!("index out of bounds"),
                }
            }

            fn draw(&mut self, ctx: DrawCtx, bounds: Rect<f32>, idx: usize) {
                match idx {
                    $( $i => self.$i.draw(ctx, bounds), )+
                    _ => panic!("index out of bounds"),
                }
            }

            fn handle(&mut self, ctx: HandleCtx<D>, bounds: Rect<f32>, event: Event, idx: usize) {
                match idx {
                    $( $i => self.$i.handle(ctx, bounds, event), )+
                    _ => panic!("index out of bounds"),
                }
            }
        }

        impl<T, $( $V, )+> MetaSeq<T> for ($( $V, )+) {
            type MetaSeq = [T; $len];

            fn new_meta_seq<F>(mut ctor: F) -> Self::MetaSeq
            where
                F: FnMut() -> T,
            {
                [$( {
                    let _ = $i;
                    ctor()
                }, )+]
            }
        }
    }
}

impl_tuple!(1, 0 => V0);
impl_tuple!(2, 0 => V0, 1 => V1);
impl_tuple!(3, 0 => V0, 1 => V1, 2 => V2);
impl_tuple!(4, 0 => V0, 1 => V1, 2 => V2, 3 => V3);
