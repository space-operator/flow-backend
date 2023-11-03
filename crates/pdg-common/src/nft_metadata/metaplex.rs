use super::{EnumExt, PropertyNotFound, RenderParams};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

/// Traits that will be included when uploading to Metaplex
pub struct NftTraits {
    pub body: super::BodyType,
    pub helmet: super::HelmetType,
    pub helmet_light: super::HelmetLight,
    pub color: f64,
    pub pose: super::Pose,
    pub composition: super::Fx0,
    pub transformation: super::Fx1,
    pub season: super::Fx2,
    pub weather: super::Fx4,
    pub smoke: super::Fx3,
    pub growth: super::Fx5,
    pub wrapping: super::Fx6,
    pub animal: Animal,
    pub seed: i64,
}

#[derive(strum::EnumProperty, strum::EnumIter, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Animal {
    #[strum(props(MetaplexName = "No"))]
    No,
    #[strum(props(MetaplexName = "Jellyfish"))]
    Jellyfish,
    #[strum(props(MetaplexName = "Firefly"))]
    Firefly,
    #[strum(props(MetaplexName = "Ladybug"))]
    Ladybug,
    #[strum(props(MetaplexName = "Butterfly"))]
    Butterfly,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MetaplexAttribute {
    pub trait_type: String,
    pub value: String,
}

#[derive(ThisError, Debug)]
#[error("{:?}", self)]
pub enum ParseMetaflexError {
    TraitNotFound {
        trait_type: String,
    },
    UnknownVariant {
        ty: &'static str,
        value: String,
    },
    PropertyNotFound(#[from] PropertyNotFound),
    ParsingError {
        trait_type: String,
        value: String,
        error: String,
    },
}

impl NftTraits {
    pub fn new(r: &RenderParams) -> Self {
        Self {
            body: r.body_type,
            helmet: r.helmet_type,
            helmet_light: r.helmet_light,
            color: r.dress_color_hue.round(),
            pose: r.pose,
            composition: r.fx0,
            transformation: r.fx1,
            season: r.fx2,
            weather: r.fx4,
            smoke: r.fx3,
            growth: r.fx5,
            wrapping: r.fx6,
            animal: match r.fx2 {
                super::Fx2::No => Animal::No,
                super::Fx2::Butterflies => Animal::Butterfly,
                super::Fx2::Underwater => match r.fx_jellifish {
                    super::FxJellyfish::No => Animal::No,
                    super::FxJellyfish::Yes => Animal::Jellyfish,
                },
                super::Fx2::Fireflyies => Animal::Firefly,
                super::Fx2::Fall => Animal::No,
                super::Fx2::Ladybag => Animal::Ladybug,
                super::Fx2::Spring => Animal::No,
            },
            seed: r.wedgeindex,
        }
    }

    /// Read from an `attributes` array
    ///
    /// https://docs.metaplex.com/programs/token-metadata/token-standard#the-programmable-non-fungible-standard
    pub fn parse_metaflex_attrs(v: &[MetaplexAttribute]) -> Result<Self, ParseMetaflexError> {
        fn find_str<'a>(
            v: &'a [MetaplexAttribute],
            trait_type: &str,
        ) -> Result<&'a str, ParseMetaflexError> {
            v.iter()
                .find(|a| a.trait_type == trait_type)
                .map(|a| a.value.as_str())
                .ok_or_else(|| ParseMetaflexError::TraitNotFound {
                    trait_type: trait_type.to_owned(),
                })
        }
        use std::str::FromStr;
        fn find_from_str<I>(
            v: &[MetaplexAttribute],
            trait_type: &str,
        ) -> Result<I, ParseMetaflexError>
        where
            I: FromStr,
            I::Err: ToString,
        {
            v.iter()
                .find(|a| a.trait_type == trait_type)
                .map(|a| a.value.as_str())
                .ok_or_else(|| ParseMetaflexError::TraitNotFound {
                    trait_type: trait_type.to_owned(),
                })
                .and_then(|s| {
                    s.parse()
                        .map_err(|e: I::Err| ParseMetaflexError::ParsingError {
                            trait_type: trait_type.to_owned(),
                            value: s.to_owned(),
                            error: e.to_string(),
                        })
                })
        }
        fn find_enum<E: EnumExt + strum::IntoEnumIterator>(
            v: &[MetaplexAttribute],
            trait_type: &str,
        ) -> Result<E, ParseMetaflexError> {
            let s = find_str(v, trait_type)?;
            for variant in E::iter() {
                if variant.metaplex_name()? == s {
                    return Ok(variant);
                }
            }

            Err(ParseMetaflexError::UnknownVariant {
                ty: std::any::type_name::<E>(),
                value: s.to_owned(),
            })
        }
        Ok(Self {
            body: find_enum(v, "Body")?,
            helmet: find_enum(v, "Helmet")?,
            helmet_light: find_enum(v, "Helmet Light")?,
            color: find_from_str(v, "Color")?,
            pose: find_enum(v, "Pose")?,
            composition: find_enum(v, "Composition")?,
            transformation: find_enum(v, "Transformation")?,
            season: find_enum(v, "Season")?,
            weather: find_enum(v, "Weather")?,
            smoke: find_enum(v, "Smoke")?,
            growth: find_enum(v, "Growth")?,
            wrapping: find_enum(v, "Wrapping")?,
            animal: find_enum(v, "Animal")?,
            seed: find_from_str(v, "Seed")?,
        })
    }

    /// Convert into `attributes` array
    ///
    /// https://docs.metaplex.com/programs/token-metadata/token-standard#the-programmable-non-fungible-standard
    pub fn gen_metaplex_attrs(&self) -> Result<Vec<MetaplexAttribute>, PropertyNotFound> {
        let Self {
            body,
            helmet,
            helmet_light,
            color,
            pose,
            composition,
            transformation,
            season,
            weather,
            smoke,
            growth,
            wrapping,
            animal,
            seed,
        } = self;

        fn push(v: &mut Vec<MetaplexAttribute>, ty: &str, value: impl Into<String>) {
            assert!(v.iter().all(|a| a.trait_type != ty));
            v.push(MetaplexAttribute {
                trait_type: ty.to_owned(),
                value: value.into(),
            });
        }

        let mut v = Vec::new();

        push(&mut v, "Body", body.metaplex_name()?);
        push(&mut v, "Helmet", helmet.metaplex_name()?);
        push(&mut v, "Helmet Light", helmet_light.metaplex_name()?);
        push(&mut v, "Color", color.round().to_string());
        push(&mut v, "Pose", pose.metaplex_name()?);
        push(&mut v, "Composition", composition.metaplex_name()?);
        push(&mut v, "Transformation", transformation.metaplex_name()?);
        push(&mut v, "Season", season.metaplex_name()?);
        push(&mut v, "Weather", weather.metaplex_name()?);
        push(&mut v, "Smoke", smoke.metaplex_name()?);
        push(&mut v, "Growth", growth.metaplex_name()?);
        push(&mut v, "Wrapping", wrapping.metaplex_name()?);
        push(&mut v, "Animal", animal.metaplex_name()?);
        push(&mut v, "Seed", seed.to_string());

        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nft_metadata::{
        BodyType, EnumExt, Fx0, Fx1, Fx2, Fx3, Fx4, Fx5, Fx6, HelmetLight, HelmetType, Pose,
        RenderParams,
    };
    use strum::{EnumProperty, IntoEnumIterator};

    #[test]
    fn test_name_available() {
        fn test<E>()
        where
            E: IntoEnumIterator + EnumProperty + std::fmt::Debug,
        {
            for variant in E::iter() {
                variant.metaplex_name().unwrap();
            }
        }
        test::<BodyType>();
        test::<HelmetType>();
        test::<HelmetLight>();
        test::<Pose>();
        test::<Fx0>();
        test::<Fx1>();
        test::<Fx2>();
        test::<Fx3>();
        test::<Fx4>();
        test::<Fx5>();
        test::<Fx6>();
        test::<Animal>();
    }

    #[test]
    fn test_gen_metaflex_attrs() {
        let mut json =
            serde_json::from_str::<serde_json::Value>(include_str!("tests/123.json")).unwrap();
        let params = RenderParams::from_pdg_metadata(&mut json, true).unwrap();
        let meta = NftTraits::new(&params);
        let attrs = meta.gen_metaplex_attrs().unwrap();
        let json = serde_json::to_string_pretty(&attrs).unwrap();
        println!("{}", json);
    }
}
