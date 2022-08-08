use std::convert::{AsMut, AsRef};

use gg_math::{Rect, Vec2};

use crate::{DrawCtx, Event, HandleCtx, LayoutCtx, LayoutHints, View};

pub trait ViewSeq<D> {
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn update(&mut self, old: &mut Self, idx: usize) -> bool;

    fn pre_layout(&mut self, ctx: LayoutCtx, idx: usize) -> LayoutHints;

    fn layout(&mut self, ctx: LayoutCtx, size: Vec2<f32>, idx: usize) -> Vec2<f32>;

    fn draw(&mut self, ctx: DrawCtx, bounds: Rect<f32>, idx: usize);

    fn handle(&mut self, ctx: HandleCtx<D>, bounds: Rect<f32>, event: Event, idx: usize);
}

impl<D> ViewSeq<D> for () {
    fn len(&self) -> usize {
        0
    }

    fn update(&mut self, _: &mut Self, _: usize) -> bool {
        false
    }

    fn pre_layout(&mut self, _: LayoutCtx, _: usize) -> LayoutHints {
        LayoutHints::default()
    }

    fn layout(&mut self, _: LayoutCtx, size: Vec2<f32>, _: usize) -> Vec2<f32> {
        size
    }

    fn draw(&mut self, _: DrawCtx, _: Rect<f32>, _: usize) {}

    fn handle(&mut self, _: HandleCtx<D>, _: Rect<f32>, _: Event, _: usize) {}
}

impl<D, VS, V> ViewSeq<D> for (V, VS)
where
    VS: ViewSeq<D>,
    V: View<D>,
{
    fn len(&self) -> usize {
        1 + self.1.len()
    }

    fn update(&mut self, old: &mut Self, idx: usize) -> bool {
        if idx == 0 {
            self.0.update(&mut old.0)
        } else {
            self.1.update(&mut old.1, idx - 1)
        }
    }

    fn pre_layout(&mut self, ctx: LayoutCtx, idx: usize) -> LayoutHints {
        if idx == 0 {
            self.0.pre_layout(ctx)
        } else {
            self.1.pre_layout(ctx, idx - 1)
        }
    }

    fn layout(&mut self, ctx: LayoutCtx, size: Vec2<f32>, idx: usize) -> Vec2<f32> {
        if idx == 0 {
            self.0.layout(ctx, size)
        } else {
            self.1.layout(ctx, size, idx - 1)
        }
    }

    fn draw(&mut self, ctx: DrawCtx, bounds: Rect<f32>, idx: usize) {
        if idx == 0 {
            self.0.draw(ctx, bounds)
        } else {
            self.1.draw(ctx, bounds, idx - 1)
        }
    }

    fn handle(&mut self, ctx: HandleCtx<D>, bounds: Rect<f32>, event: Event, idx: usize) {
        if idx == 0 {
            self.0.handle(ctx, bounds, event)
        } else {
            self.1.handle(ctx, bounds, event, idx - 1)
        }
    }
}

pub trait Append<T> {
    type Output;

    fn append(self, rhs: T) -> Self::Output;
}

impl<A> Append<A> for () {
    type Output = (A, ());

    fn append(self, rhs: A) -> Self::Output {
        (rhs, ())
    }
}

impl<V, VS, A> Append<A> for (V, VS)
where
    VS: Append<A>,
{
    type Output = (V, <VS as Append<A>>::Output);

    fn append(self, rhs: A) -> Self::Output {
        (self.0, self.1.append(rhs))
    }
}

pub trait HasMetaSeq<T> {
    type MetaSeq: AsRef<[T]> + AsMut<[T]>;

    fn new_meta_seq<F: FnMut() -> T>(ctor: F) -> Self::MetaSeq;
}

impl<T> HasMetaSeq<T> for () {
    type MetaSeq = tuple_meta::Empty;

    fn new_meta_seq<F: FnMut() -> T>(_: F) -> Self::MetaSeq {
        tuple_meta::Empty
    }
}

impl<T, V, VS> HasMetaSeq<T> for (V, VS)
where
    VS: HasMetaSeq<T>,
    VS::MetaSeq: tuple_meta::Seq<T>,
{
    type MetaSeq = tuple_meta::Cons<T, VS::MetaSeq>;

    fn new_meta_seq<F: FnMut() -> T>(mut ctor: F) -> Self::MetaSeq {
        tuple_meta::Cons {
            head: ctor(),
            tail: VS::new_meta_seq(ctor),
        }
    }
}

mod tuple_meta {
    use super::*;

    pub trait Seq<T> {
        fn len(&self) -> usize;
    }

    #[repr(C)]
    pub struct Empty;

    impl<T> Seq<T> for Empty {
        fn len(&self) -> usize {
            0
        }
    }

    impl<T> AsRef<[T]> for Empty {
        fn as_ref(&self) -> &[T] {
            &[]
        }
    }

    impl<T> AsMut<[T]> for Empty {
        fn as_mut(&mut self) -> &mut [T] {
            &mut []
        }
    }

    #[repr(C)]
    pub struct Cons<T, TS: Seq<T>> {
        pub head: T,
        pub tail: TS,
    }

    impl<T, TS: Seq<T>> Seq<T> for Cons<T, TS> {
        fn len(&self) -> usize {
            1 + self.tail.len()
        }
    }

    impl<T, TS: Seq<T>> AsRef<[T]> for Cons<T, TS> {
        fn as_ref(&self) -> &[T] {
            let len = self.len();
            unsafe { std::slice::from_raw_parts(self as *const Self as *const T, len) }
        }
    }

    impl<T, TS: Seq<T>> AsMut<[T]> for Cons<T, TS> {
        fn as_mut(&mut self) -> &mut [T] {
            let len = self.len();
            unsafe { std::slice::from_raw_parts_mut(self as *mut Self as *mut T, len) }
        }
    }
}

pub trait IntoViewSeq<D> {
    type ViewSeq: ViewSeq<D>;

    fn into_view_seq(self) -> Self::ViewSeq;
}

macro_rules! impl_tuple {
    () => {
        impl_tuple!(@impl);
    };

    ($V:ident, $( $VS:ident, )*) => {
        impl_tuple!($( $VS, )*);
        impl_tuple!(@impl $V, $( $VS, )*);
    };

    (@impl $( $VS:ident, )*) => {
        impl<D, $( $VS, )*> IntoViewSeq<D> for ($( $VS, )*)
        where
            $($VS: View<D>, )*
        {
            type ViewSeq = impl_tuple!(@cons $( $VS, )*);

            #[allow(non_snake_case)]
            fn into_view_seq(self) -> Self::ViewSeq {
                let ($( $VS, )*) = self;
                impl_tuple!(@cons $( $VS, )*)
            }
        }
    };

    (@cons) => {
        ()
    };

    (@cons $V:ident,) => {
        ($V, ())
    };

    (@cons $V:ident, $( $VS:ident, )+) => {
        ($V, impl_tuple!(@cons $( $VS, )+))
    };
}

impl_tuple!(V0, V1, V2, V3, V4, V5, V6, V7, V8, V9,);
