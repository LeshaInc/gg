use gg_assets::{Assets, Handle};
use gg_util::ahash::AHashMap;

use super::{FontStyle, FontWeight};
use crate::{FontCollection, FontFace};

#[derive(Debug, Default)]
pub struct FontDb {
    map: AHashMap<String, Vec<Variant>>,
    new_faces: Vec<Handle<FontFace>>,
    new_collections: Vec<Handle<FontCollection>>,
}

#[derive(Debug, Eq, PartialEq)]
struct Variant {
    weight: FontWeight,
    style: FontStyle,
    face: Handle<FontFace>,
}

impl FontDb {
    pub fn new() -> FontDb {
        FontDb::default()
    }

    pub fn add_face(&mut self, face: &Handle<FontFace>) {
        self.new_faces.push(face.clone());
    }

    pub fn add_collection(&mut self, collection: &Handle<FontCollection>) {
        self.new_collections.push(collection.clone());
    }

    pub fn update(&mut self, assets: &Assets) {
        let mut i = 0;
        while i < self.new_collections.len() {
            let handle = &self.new_collections[i];
            if let Some(collection) = assets.get(handle) {
                for face in &collection.faces {
                    self.new_faces.push(face.clone());
                }

                self.new_collections.remove(i);
            } else {
                i += 1;
            }
        }

        let mut i = 0;
        while i < self.new_faces.len() {
            let handle = &self.new_faces[i];
            if let Some(face) = assets.get(handle) {
                let props = face.props();

                let variant = Variant {
                    weight: props.weight,
                    style: props.style,
                    face: handle.clone(),
                };

                if !self.map.contains_key(&props.name) {
                    self.map.insert(props.name.clone(), Vec::new());
                }

                let variants = self.map.get_mut(&props.name).unwrap();
                if !variants.contains(&variant) {
                    variants.push(variant);
                }

                self.new_faces.remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn find(
        &self,
        name: &str,
        weight: FontWeight,
        style: FontStyle,
    ) -> Option<&Handle<FontFace>> {
        let variants = self.map.get(name)?.iter();
        variants
            .min_by_key(|v| style_diff(v.style, style) + weight_diff(v.weight, weight))
            .map(|v| &v.face)
    }
}

fn style_diff(a: FontStyle, b: FontStyle) -> u16 {
    match (a, b) {
        _ if a == b => 0,
        (FontStyle::Oblique, FontStyle::Italic) => 10000,
        (FontStyle::Italic, FontStyle::Oblique) => 10000,
        _ => 20000,
    }
}

fn weight_diff(a: FontWeight, b: FontWeight) -> u16 {
    let (a, b) = (a.to_number(), b.to_number());
    if a > b {
        a - b
    } else {
        b - a
    }
}
