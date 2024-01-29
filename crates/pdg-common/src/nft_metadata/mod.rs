use self::pdg::{Attr, AttrCfg};
use rand::{
    distributions::{Uniform, WeightedIndex},
    prelude::Distribution,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::{borrow::Cow, fmt::Debug};
use strum::{Display, EnumProperty, IntoEnumIterator};
use thiserror::Error as ThisError;

pub mod generate;
pub mod metaplex;
pub mod pdg;

#[derive(ThisError, Debug)]
#[error("{:?}", self)]
pub struct PropertyNotFound {
    pub attr: &'static str,
    pub ty: &'static str,
    pub variant: String,
}

trait EnumExt {
    fn pdg_name(&self) -> Result<&'static str, PropertyNotFound>;
    fn metaplex_name(&self) -> Result<&'static str, PropertyNotFound>;
    fn effect_type(&self) -> Result<&'static str, PropertyNotFound>;
}

impl<T> EnumExt for T
where
    T: strum::EnumProperty + std::fmt::Debug,
{
    fn pdg_name(&self) -> Result<&'static str, PropertyNotFound> {
        self.get_str("PDGName").ok_or_else(|| PropertyNotFound {
            attr: "PDGName",
            ty: std::any::type_name::<T>(),
            variant: format!("{:?}", self),
        })
    }
    fn metaplex_name(&self) -> Result<&'static str, PropertyNotFound> {
        self.get_str("MetaplexName")
            .ok_or_else(|| PropertyNotFound {
                attr: "MetaplexName",
                ty: std::any::type_name::<T>(),
                variant: format!("{:?}", self),
            })
    }
    fn effect_type(&self) -> Result<&'static str, PropertyNotFound> {
        self.get_str("EffectType").ok_or_else(|| PropertyNotFound {
            attr: "EffectType",
            ty: std::any::type_name::<T>(),
            variant: format!("{:?}", self),
        })
    }
}

/*
const DEFAULT_SPLIT: i64 = 1;
const DEFAULT_WEDGECOUNT: i64 = 30;
const DEFAULT_WEDGENUM: i64 = 0;
const DEFAULT_WEDGETOTAL: i64 = 30;

const DEFAULT_WEDGEATTRIBS: Attr<&[&str]> = Attr {
    cfg: AttrCfg::new_type(2),
    value: &[
        "Body_type",
        "Butterfly_amount",
        "Desintegration_amount",
        "Env_Light",
        "Env_reflection",
        "Eyes_light_intensity_amount",
        "FX_lineart_helper",
        "Fall_amount",
        "Firefly_amount",
        "Frozen_amount",
        "Fungi_amount",
        "Fx_Jellifish",
        "Fx_switcher_layer_0",
        "Fx_switcher_layer_1",
        "Fx_switcher_layer_1a",
        "Fx_switcher_layer_2",
        "Fx_switcher_layer_3",
        "Fx_switcher_layer_4",
        "Fx_switcher_layer_5",
        "Fx_switcher_layer_6",
        "Gold_silver_amount",
        "Grow_flower_amount",
        "Helmet_light",
        "Helmet_type",
        "Hologram_amount",
        "Ladybag_amount",
        "Lineart_amount",
        "Melt_amount",
        "Melting_glow_amount",
        "Pixel_amount",
        "Pose",
        "Rain_amount",
        "Render_noise_threshold",
        "Render_resolution",
        "Smoke_amount",
        "Soap_bubble_intensity_amount",
        "Soap_bubble_roughness_amount",
        "Spring_amount",
        "Underwater_fog_amount",
        "Xray_body_amount",
        "Xray_skeleton_particles_amount",
        "background_color_random_hue",
        "background_underwater_color_hue",
        "dress_color_hue",
        "eye_color_random_hue",
        "light_reflection_mult",
        "random_value",
    ],
};
*/

/// Condensed metadata
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RenderParams {
    pub body_type: BodyType,
    pub pose: Pose,
    pub helmet_type: HelmetType,
    pub helmet_light: HelmetLight,
    pub fx0: Fx0,
    pub fx1: Fx1,
    pub fx1a: Fx1a,
    pub fx2: Fx2,
    pub fx3: Fx3,
    pub fx4: Fx4,
    pub fx5: Fx5,
    pub fx6: Fx6,

    pub fx0_bodyoff: Fx0BodyOff,
    pub fx0_bodyoff_glass: Fx0BodyOffGlass,
    pub body_material_variation: BodyMaterialVariations,
    pub marble_variation: MarbleVariation,
    pub wood_variation: WoodVariation,

    pub fx_jellifish: FxJellyfish,
    pub fx_lineart_helper: FxLineartHelper,
    pub env_light: EnvLight,
    pub env_reflection: EnvReflection,
    pub light_reflection_mult: LightReflectionMult,

    pub glowing_logo: GlowingLogo,
    pub logo_hue: f64,
    pub logo_name: String,

    pub butterfly_amount: f64,
    pub disintegration_amount: f64,
    pub melt_amount: f64,
    pub fall_amount: f64,
    pub firefly_amount: f64,
    pub frozen_amount: f64,
    pub fungi_amount: f64,
    pub gold_silver_amount: f64,
    pub grow_flower_amount: f64,
    pub hologram_amount: f64,
    pub eyes_light_intensity_amount: f64,
    pub ladybag_amount: f64,
    pub lineart_amount: f64,
    pub melting_glow_amount: f64,
    pub pixel_amount: f64,
    pub rain_amount: f64,
    pub smoke_amount: f64,
    pub soap_bubble_intensity_amount: f64,
    pub soap_bubble_roughness_amount: f64,
    pub spring_amount: f64,
    pub underwater_fog_amount: f64,
    pub xray_body_amount: f64,
    pub xray_skeleton_particles_amount: f64,

    pub background_color_random_hue: f64,
    pub background_underwater_color_hue: f64,
    pub dress_color_hue: f64,
    pub eye_color_random_hue: f64,

    pub random_value: f64,
    pub wedgeindex: i64,

    pub render_noise_threshold: f64,
    pub render_resolution: u32,
    pub wedgeattribs: Vec<String>,
}

// add a empty default
impl Default for RenderParams {
    fn default() -> Self {
        Self {
            body_type: BodyType::default(),
            pose: Pose::default(),
            helmet_type: HelmetType::default(),
            helmet_light: HelmetLight::default(),
            fx0: Fx0::default(),
            fx1: Fx1::default(),
            fx1a: Fx1a::default(),
            fx2: Fx2::default(),
            fx3: Fx3::default(),
            fx4: Fx4::default(),
            fx5: Fx5::default(),
            fx6: Fx6::default(),
            fx0_bodyoff: Fx0BodyOff::default(),
            fx0_bodyoff_glass: Fx0BodyOffGlass::default(),
            body_material_variation: BodyMaterialVariations::default(),
            marble_variation: MarbleVariation::default(),
            wood_variation: WoodVariation::default(),
            fx_jellifish: FxJellyfish::default(),
            fx_lineart_helper: FxLineartHelper::default(),
            env_light: EnvLight::default(),
            env_reflection: EnvReflection::default(),
            light_reflection_mult: LightReflectionMult::default(),
            glowing_logo: GlowingLogo::default(),
            logo_hue: 0.0,
            logo_name: "solana.png".to_owned(),
            butterfly_amount: 0.0,
            disintegration_amount: 0.0,
            melt_amount: 0.0,
            fall_amount: 0.0,
            firefly_amount: 0.0,
            frozen_amount: 0.0,
            fungi_amount: 0.0,
            gold_silver_amount: 0.0,
            grow_flower_amount: 0.0,
            hologram_amount: 0.0,
            eyes_light_intensity_amount: 0.0,
            ladybag_amount: 0.0,
            lineart_amount: 0.0,
            melting_glow_amount: 0.0,
            pixel_amount: 0.0,
            rain_amount: 0.0,
            smoke_amount: 0.0,
            soap_bubble_intensity_amount: 0.0,
            soap_bubble_roughness_amount: 0.0,
            spring_amount: 0.0,
            underwater_fog_amount: 0.0,
            xray_body_amount: 0.0,
            xray_skeleton_particles_amount: 0.0,
            background_color_random_hue: 0.0,
            background_underwater_color_hue: 0.0,
            dress_color_hue: 0.0,
            eye_color_random_hue: 0.0,
            random_value: 0.0,
            wedgeindex: 5043,
            render_noise_threshold: 0.6,
            render_resolution: 1024,
            wedgeattribs: [
                "Body_type".to_owned(),
                "Butterfly_amount".to_owned(),
                "Desintegration_amount".to_owned(),
                "Env_Light".to_owned(),
                "Env_reflection".to_owned(),
                "Eyes_light_intensity_amount".to_owned(),
                "FX_lineart_helper".to_owned(),
                "Fall_amount".to_owned(),
                "Firefly_amount".to_owned(),
                "Frozen_amount".to_owned(),
                "Fungi_amount".to_owned(),
                "Fx_Jellifish".to_owned(),
                "Fx_switcher_layer_0".to_owned(),
                "Fx_switcher_layer_1".to_owned(),
                "Fx_switcher_layer_1a".to_owned(),
                "Fx_switcher_layer_2".to_owned(),
                "Fx_switcher_layer_3".to_owned(),
                "Fx_switcher_layer_4".to_owned(),
                "Fx_switcher_layer_5".to_owned(),
                "Fx_switcher_layer_6".to_owned(),
                "Gold_silver_amount".to_owned(),
                "Grow_flower_amount".to_owned(),
                "Helmet_light".to_owned(),
                "Helmet_type".to_owned(),
                "Hologram_amount".to_owned(),
                "Ladybag_amount".to_owned(),
                "Lineart_amount".to_owned(),
                "Melt_amount".to_owned(),
                "Melting_glow_amount".to_owned(),
                "Pixel_amount".to_owned(),
                "Pose".to_owned(),
                "Rain_amount".to_owned(),
                "Render_noise_threshold".to_owned(),
                "Render_resolution".to_owned(),
                "Smoke_amount".to_owned(),
                "Soap_bubble_intensity_amount".to_owned(),
                "Soap_bubble_roughness_amount".to_owned(),
                "Spring_amount".to_owned(),
                "Underwater_fog_amount".to_owned(),
                "Xray_body_amount".to_owned(),
                "Xray_skeleton_particles_amount".to_owned(),
                "background_color_random_hue".to_owned(),
                "background_underwater_color_hue".to_owned(),
                "dress_color_hue".to_owned(),
                "eye_color_random_hue".to_owned(),
                "light_reflection_mult".to_owned(),
                "random_value".to_owned(),
            ]
            .into(),
        }
    }
}

fn not_found(path: impl Into<Cow<'static, str>>) -> FromPDGError {
    FromPDGError::NotFound(path.into())
}

fn unknown_variant(ty: &'static str, var: u32) -> FromPDGError {
    FromPDGError::UnknownVariant(ty, var)
}

#[derive(ThisError, Debug)]
#[error("{:?}", self)]
pub enum FromPDGError {
    DifferentConfig(AttrCfg),
    ExpectedObject,
    NotFound(Cow<'static, str>),
    UnknownVariant(&'static str, u32),
    Json(#[from] serde_json::Error),
    WrongName {
        path: &'static str,
        expected: &'static str,
        got: String,
    },
    UnexpectedValue {
        path: &'static str,
        expected: String,
        got: String,
    },
    PropertyNotFound(#[from] PropertyNotFound),
}

impl RenderParams {
    pub fn from_pdg_metadata(
        m: &mut serde_json::Value,
        check_human_readable: bool,
    ) -> Result<Self, FromPDGError> {
        fn try_get_enum<E: TryFrom<u32, Error = FromPDGError>>(
            m: &mut serde_json::Value,
            path: &'static str,
        ) -> Result<E, FromPDGError> {
            let json = m
                .as_object_mut()
                .ok_or_else(|| FromPDGError::ExpectedObject)?
                .remove(path)
                .ok_or_else(|| not_found(path))?;
            let attr = serde_json::from_value::<Attr<(u32,)>>(json)?;
            if attr.cfg != AttrCfg::new_type(0) {
                return Err(FromPDGError::DifferentConfig(attr.cfg));
            }
            E::try_from(attr.value.0)
        }

        fn check_enum_name(
            m: &mut serde_json::Value,
            path: &'static str,
            variant_name: &'static str,
        ) -> Result<(), FromPDGError> {
            let json = m
                .as_object_mut()
                .ok_or_else(|| FromPDGError::ExpectedObject)?
                .remove(path)
                .ok_or_else(|| not_found(path))?;
            let attr = serde_json::from_value::<Attr<(String,)>>(json)?;
            if attr.cfg != AttrCfg::new_type(2) {
                return Err(FromPDGError::DifferentConfig(attr.cfg));
            }
            if attr.value.0 != variant_name {
                return Err(FromPDGError::WrongName {
                    path,
                    expected: variant_name,
                    got: attr.value.0,
                });
            }
            Ok(())
        }

        fn try_get_f64(m: &mut serde_json::Value, path: &'static str) -> Result<f64, FromPDGError> {
            let json = m
                .as_object_mut()
                .ok_or_else(|| FromPDGError::ExpectedObject)?
                .remove(path)
                .ok_or_else(|| not_found(path))?;
            let attr = serde_json::from_value::<Attr<(f64,)>>(json)?;
            if attr.cfg != AttrCfg::new_type(1) {
                return Err(FromPDGError::DifferentConfig(attr.cfg));
            }
            Ok(attr.value.0)
        }

        fn try_get_int<I: DeserializeOwned>(
            m: &mut serde_json::Value,
            path: &'static str,
        ) -> Result<I, FromPDGError> {
            let json = m
                .as_object_mut()
                .ok_or_else(|| FromPDGError::ExpectedObject)?
                .remove(path)
                .ok_or_else(|| not_found(path))?;
            let attr = serde_json::from_value::<Attr<(I,)>>(json)?;
            if attr.cfg != AttrCfg::new_type(0) {
                return Err(FromPDGError::DifferentConfig(attr.cfg));
            }
            Ok(attr.value.0)
        }
        fn try_get_string(
            m: &mut serde_json::Value,
            path: &'static str,
        ) -> Result<String, FromPDGError> {
            let json = m
                .as_object_mut()
                .ok_or_else(|| FromPDGError::ExpectedObject)?
                .remove(path)
                .ok_or_else(|| not_found(path))?;
            let attr = serde_json::from_value::<Attr<(String,)>>(json)?;
            if attr.cfg != AttrCfg::new_type(2) {
                return Err(FromPDGError::DifferentConfig(attr.cfg));
            }
            Ok(attr.value.0)
        }

        let body_type = try_get_enum::<BodyType>(m, "Body_type")?;
        if check_human_readable {
            check_enum_name(m, "Body_name", body_type.pdg_name()?)?;
        }
        let pose = try_get_enum::<Pose>(m, "Pose")?;
        if check_human_readable {
            check_enum_name(m, "Pose_name", pose.pdg_name()?)?;
        }
        let helmet_type = try_get_enum::<HelmetType>(m, "Helmet_type")?;
        if check_human_readable {
            check_enum_name(m, "Helmet_name", helmet_type.pdg_name()?)?;
        }
        let helmet_light = try_get_enum::<HelmetLight>(m, "Helmet_light")?;
        if check_human_readable {
            check_enum_name(m, "Helmet_Light_name", helmet_light.pdg_name()?)?;
        }

        let fx0 = try_get_enum::<Fx0>(m, "Fx_switcher_layer_0")?;
        if check_human_readable {
            check_enum_name(m, "Fx_0", fx0.pdg_name()?)?;
        }

        let fx1 = try_get_enum::<Fx1>(m, "Fx_switcher_layer_1")?;
        if check_human_readable {
            check_enum_name(m, "Fx_1", fx1.pdg_name()?)?;
        }

        let fx1a = try_get_enum::<Fx1a>(m, "Fx_switcher_layer_1a")?;
        if check_human_readable {
            check_enum_name(m, "Fx_1a", fx1a.pdg_name()?)?;
        }

        let fx2 = try_get_enum::<Fx2>(m, "Fx_switcher_layer_2")?;
        if check_human_readable {
            check_enum_name(m, "Fx_2", fx2.pdg_name()?)?;
        }

        let fx3 = try_get_enum::<Fx3>(m, "Fx_switcher_layer_3")?;
        if check_human_readable {
            check_enum_name(m, "Fx_3", fx3.pdg_name()?)?;
        }

        let fx4 = try_get_enum::<Fx4>(m, "Fx_switcher_layer_4")?;
        if check_human_readable {
            check_enum_name(m, "Fx_4", fx4.pdg_name()?)?;
        }

        let fx5 = try_get_enum::<Fx5>(m, "Fx_switcher_layer_5")?;
        if check_human_readable {
            check_enum_name(m, "Fx_5", fx5.pdg_name()?)?;
        }

        let fx6 = try_get_enum::<Fx6>(m, "Fx_switcher_layer_6")?;
        if check_human_readable {
            check_enum_name(m, "Fx_6", fx6.pdg_name()?)?;
        }

        let fx0_bodyoff = try_get_enum::<Fx0BodyOff>(m, "Fx_bodyoff_layer_0_1_1a")?;
        let fx0_bodyoff_glass =
            try_get_enum::<Fx0BodyOffGlass>(m, "Fx_bodyoff_layer_0_1_1a_glass")?;

        let body_material_variation =
            try_get_enum::<BodyMaterialVariations>(m, "Body_material_variation")?;

        let marble_variation = try_get_enum::<MarbleVariation>(m, "Marble_variation")?;

        let wood_variation = try_get_enum::<WoodVariation>(m, "Wood_variation")?;

        let fx_jellifish = try_get_enum::<FxJellyfish>(m, "Fx_Jellifish")?;
        if check_human_readable {
            check_enum_name(m, "Jellifish", fx_jellifish.pdg_name()?)?;
        }

        let fx_lineart_helper = try_get_enum::<FxLineartHelper>(m, "FX_lineart_helper")?;

        let env_light = try_get_enum::<EnvLight>(m, "Env_Light")?;

        let env_reflection = try_get_enum::<EnvReflection>(m, "Env_reflection")?;

        let light_reflection_mult =
            try_get_enum::<LightReflectionMult>(m, "light_reflection_mult")?;

        let glowing_logo = try_get_enum::<GlowingLogo>(m, "Glowing_logo")?;
        let logo_hue = try_get_f64(m, "Logo_hue")?;
        let logo_name = try_get_string(m, "logo_name")?;

        let butterfly_amount = try_get_f64(m, "Butterfly_amount")?;
        let disintegration_amount = try_get_f64(m, "Desintegration_amount")?;
        let melt_amount = try_get_f64(m, "Melt_amount")?;
        let fall_amount = try_get_f64(m, "Fall_amount")?;
        let firefly_amount = try_get_f64(m, "Firefly_amount")?;
        let frozen_amount = try_get_f64(m, "Frozen_amount")?;
        let fungi_amount = try_get_f64(m, "Fungi_amount")?;
        let gold_silver_amount = try_get_f64(m, "Gold_silver_amount")?;
        let grow_flower_amount = try_get_f64(m, "Grow_flower_amount")?;
        let hologram_amount = try_get_f64(m, "Hologram_amount")?;
        let eyes_light_intensity_amount = try_get_f64(m, "Eyes_light_intensity_amount")?;
        let ladybag_amount = try_get_f64(m, "Ladybag_amount")?;
        let lineart_amount = try_get_f64(m, "Lineart_amount")?;
        let melting_glow_amount = try_get_f64(m, "Melting_glow_amount")?;
        let pixel_amount = try_get_f64(m, "Pixel_amount")?;
        let rain_amount = try_get_f64(m, "Rain_amount")?;
        let smoke_amount = try_get_f64(m, "Smoke_amount")?;
        let soap_bubble_intensity_amount = try_get_f64(m, "Soap_bubble_intensity_amount")?;
        let soap_bubble_roughness_amount = try_get_f64(m, "Soap_bubble_roughness_amount")?;
        let spring_amount = try_get_f64(m, "Spring_amount")?;
        let underwater_fog_amount = try_get_f64(m, "Underwater_fog_amount")?;
        let xray_body_amount = try_get_f64(m, "Xray_body_amount")?;
        let xray_skeleton_particles_amount = try_get_f64(m, "Xray_skeleton_particles_amount")?;

        let background_color_random_hue = try_get_f64(m, "background_color_random_hue")?;
        let background_underwater_color_hue = try_get_f64(m, "background_underwater_color_hue")?;
        let dress_color_hue = try_get_f64(m, "dress_color_hue")?;
        let eye_color_random_hue = try_get_f64(m, "eye_color_random_hue")?;

        let random_value = try_get_f64(m, "random_value")?;

        let wedgeindex = try_get_int::<i64>(m, "wedgeindex")?;

        let render_noise_threshold = try_get_f64(m, "Render_noise_threshold")?;
        let render_resolution = try_get_int::<u32>(m, "Render_resolution")?;

        /*
        fn check_int<I: DeserializeOwned + PartialEq + std::fmt::Display>(
            m: &mut serde_json::Value,
            path: &'static str,
            expected: I,
        ) -> Result<(), FromPDGError> {
            let got = try_get_int::<I>(m, path)?;
            if got != expected {
                Err(FromPDGError::UnexpectedValue {
                    path,
                    expected: expected.to_string(),
                    got: got.to_string(),
                })
            } else {
                Ok(())
            }
        }

        let split = check_int::<i64>(m, "split", DEFAULT_SPLIT)?;
        check_int::<i64>(m, "wedgecount", DEFAULT_WEDGECOUNT)?;
        check_int::<i64>(m, "wedgenum", DEFAULT_WEDGENUM)?;
        check_int::<i64>(m, "wedgetotal", DEFAULT_WEDGETOTAL)?;

        let wedgeattribs = {
            let json = m
                .as_object_mut()
                .ok_or_else(|| FromPDGError::ExpectedObject)?
                .remove("wedgeattribs")
                .ok_or_else(|| not_found("wedgeattribs"))?;
            let attr = serde_json::from_value::<Attr<Vec<String>>>(json)?;
            if attr.cfg != AttrCfg::new_type(2) {
                return Err(FromPDGError::DifferentConfig(attr.cfg));
            }
            attr.value
        };
        if wedgeattribs != DEFAULT_WEDGEATTRIBS.value {
            return Err(FromPDGError::UnexpectedValue {
                path: "wedgeattribs",
                expected: format!("{:?}", DEFAULT_WEDGEATTRIBS),
                got: format!("{:?}", wedgeattribs),
            });
        }
        */

        Ok(Self {
            body_type,
            pose,
            helmet_type,
            helmet_light,
            fx0,
            fx1,
            fx1a,
            fx2,
            fx3,
            fx4,
            fx5,
            fx6,
            fx0_bodyoff,
            fx0_bodyoff_glass,
            body_material_variation,
            marble_variation,
            wood_variation,
            fx_jellifish,
            fx_lineart_helper,
            env_light,
            env_reflection,
            light_reflection_mult,
            glowing_logo,
            logo_hue,
            logo_name,
            butterfly_amount,
            disintegration_amount,
            melt_amount,
            fall_amount,
            firefly_amount,
            frozen_amount,
            fungi_amount,
            gold_silver_amount,
            grow_flower_amount,
            hologram_amount,
            eyes_light_intensity_amount,
            ladybag_amount,
            lineart_amount,
            melting_glow_amount,
            pixel_amount,
            rain_amount,
            smoke_amount,
            soap_bubble_intensity_amount,
            soap_bubble_roughness_amount,
            spring_amount,
            underwater_fog_amount,
            xray_body_amount,
            xray_skeleton_particles_amount,
            background_color_random_hue,
            background_underwater_color_hue,
            dress_color_hue,
            eye_color_random_hue,
            random_value,
            wedgeindex,
            render_noise_threshold,
            render_resolution,
            ..<_>::default()
        })
    }

    pub fn to_pdg_metadata(&self, human_readable: bool) -> Result<serde_json::Value, FromPDGError> {
        fn push_string_attr(
            m: &mut serde_json::Map<String, serde_json::Value>,
            path: &str,
            value: &str,
        ) {
            m.insert(
                path.to_owned(),
                serde_json::to_value(Attr::<(String,)> {
                    cfg: AttrCfg::new_type(2),
                    value: (value.to_owned(),),
                })
                .unwrap(),
            );
        }

        fn push_string_array_attr(
            m: &mut serde_json::Map<String, serde_json::Value>,
            path: &str,
            value: &[String],
        ) {
            m.insert(
                path.to_owned(),
                serde_json::to_value(Attr::<(Vec<String>,)> {
                    cfg: AttrCfg::new_type(2),
                    value: (value.to_vec(),),
                })
                .unwrap(),
            );
        }

        fn push_int_attr(
            m: &mut serde_json::Map<String, serde_json::Value>,
            path: &str,
            value: impl Into<i64>,
        ) {
            m.insert(
                path.to_owned(),
                serde_json::to_value(Attr::<(i64,)> {
                    cfg: AttrCfg::new_type(0),
                    value: (value.into(),),
                })
                .unwrap(),
            );
        }

        fn push_float_attr(
            m: &mut serde_json::Map<String, serde_json::Value>,
            path: &str,
            value: f64,
        ) {
            m.insert(
                path.to_owned(),
                serde_json::to_value(Attr::<(f64,)> {
                    cfg: AttrCfg::new_type(1),
                    value: (value,),
                })
                .unwrap(),
            );
        }

        let Self {
            body_type,
            pose,
            helmet_type,
            helmet_light,
            fx0,
            fx1,
            fx1a,
            fx2,
            fx3,
            fx4,
            fx5,
            fx6,
            fx0_bodyoff,
            fx0_bodyoff_glass,
            body_material_variation,
            marble_variation,
            wood_variation,
            fx_jellifish,
            fx_lineart_helper,
            env_light,
            env_reflection,
            light_reflection_mult,
            glowing_logo,
            logo_hue,
            logo_name,
            butterfly_amount,
            disintegration_amount,
            melt_amount,
            fall_amount,
            firefly_amount,
            frozen_amount,
            fungi_amount,
            gold_silver_amount,
            grow_flower_amount,
            hologram_amount,
            eyes_light_intensity_amount,
            ladybag_amount,
            lineart_amount,
            melting_glow_amount,
            pixel_amount,
            rain_amount,
            smoke_amount,
            soap_bubble_intensity_amount,
            soap_bubble_roughness_amount,
            spring_amount,
            underwater_fog_amount,
            xray_body_amount,
            xray_skeleton_particles_amount,
            background_color_random_hue,
            background_underwater_color_hue,
            dress_color_hue,
            eye_color_random_hue,
            random_value,
            wedgeindex,
            render_noise_threshold,
            render_resolution,
            wedgeattribs,
        } = &self;

        let mut m = serde_json::Map::new();

        push_string_array_attr(&mut m, "wedgeattribs", &wedgeattribs[..]);

        push_int_attr(&mut m, "Body_type", *body_type as u32);
        if human_readable {
            push_string_attr(&mut m, "Body_name", body_type.pdg_name()?);
        }

        push_int_attr(&mut m, "Pose", *pose as u32);
        if human_readable {
            push_string_attr(&mut m, "Pose_name", pose.pdg_name()?);
        }

        push_int_attr(&mut m, "Helmet_type", *helmet_type as u32);
        if human_readable {
            push_string_attr(&mut m, "Helmet_name", helmet_type.pdg_name()?);
        }

        push_int_attr(&mut m, "Helmet_light", *helmet_light as u32);
        if human_readable {
            push_string_attr(&mut m, "Helmet_Light_name", helmet_light.pdg_name()?);
        }

        push_int_attr(&mut m, "Fx_switcher_layer_0", *fx0 as u32);
        if human_readable {
            push_string_attr(&mut m, "Fx_0", fx0.pdg_name()?);
        }

        push_int_attr(&mut m, "Fx_switcher_layer_1", *fx1 as u32);
        if human_readable {
            push_string_attr(&mut m, "Fx_1", fx1.pdg_name()?);
        }

        push_int_attr(&mut m, "Fx_switcher_layer_1a", *fx1a as u32);
        if human_readable {
            push_string_attr(&mut m, "Fx_1a", fx1a.pdg_name()?);
        }

        push_int_attr(&mut m, "Fx_switcher_layer_2", *fx2 as u32);
        if human_readable {
            push_string_attr(&mut m, "Fx_2", fx2.pdg_name()?);
        }

        push_int_attr(&mut m, "Fx_switcher_layer_3", *fx3 as u32);
        if human_readable {
            push_string_attr(&mut m, "Fx_3", fx3.pdg_name()?);
        }

        push_int_attr(&mut m, "Fx_switcher_layer_4", *fx4 as u32);
        if human_readable {
            push_string_attr(&mut m, "Fx_4", fx4.pdg_name()?);
        }

        push_int_attr(&mut m, "Fx_switcher_layer_5", *fx5 as u32);
        if human_readable {
            push_string_attr(&mut m, "Fx_5", fx5.pdg_name()?);
        }

        push_int_attr(&mut m, "Fx_switcher_layer_6", *fx6 as u32);
        if human_readable {
            push_string_attr(&mut m, "Fx_6", fx6.pdg_name()?);
        }

        // Doesn't have human readable attribute
        push_int_attr(&mut m, "Fx_bodyoff_layer_0_1_1a", *fx0_bodyoff as u32);

        // Doesn't have human readable attribute
        push_int_attr(
            &mut m,
            "Fx_bodyoff_layer_0_1_1a_glass",
            *fx0_bodyoff_glass as u32,
        );

        // Doesn't have human readable attribute
        push_int_attr(
            &mut m,
            "Body_material_variation",
            *body_material_variation as u32,
        );

        // Doesn't have human readable attribute
        push_int_attr(&mut m, "Marble_variation", *marble_variation as u32);

        // Doesn't have human readable attribute
        push_int_attr(&mut m, "Wood_variation", *wood_variation as u32);

        push_int_attr(&mut m, "Fx_Jellifish", *fx_jellifish as u32);
        if human_readable {
            push_string_attr(&mut m, "Jellifish", fx_jellifish.pdg_name()?);
        }

        push_int_attr(&mut m, "FX_lineart_helper", *fx_lineart_helper as u32);

        push_int_attr(&mut m, "Env_Light", *env_light as u32);

        push_int_attr(&mut m, "Env_reflection", *env_reflection as u32);

        push_int_attr(
            &mut m,
            "light_reflection_mult",
            *light_reflection_mult as u32,
        );

        push_int_attr(&mut m, "Glowing_logo", *glowing_logo as u32);
        push_float_attr(&mut m, "Logo_hue", *logo_hue);
        push_string_attr(&mut m, "Logo_name", &logo_name);

        push_float_attr(&mut m, "Butterfly_amount", *butterfly_amount);
        push_float_attr(&mut m, "Desintegration_amount", *disintegration_amount);
        push_float_attr(&mut m, "Melt_amount", *melt_amount);
        push_float_attr(&mut m, "Fall_amount", *fall_amount);
        push_float_attr(&mut m, "Firefly_amount", *firefly_amount);
        push_float_attr(&mut m, "Frozen_amount", *frozen_amount);
        push_float_attr(&mut m, "Fungi_amount", *fungi_amount);
        push_float_attr(&mut m, "Gold_silver_amount", *gold_silver_amount);
        push_float_attr(&mut m, "Grow_flower_amount", *grow_flower_amount);
        push_float_attr(&mut m, "Hologram_amount", *hologram_amount);
        push_float_attr(
            &mut m,
            "Eyes_light_intensity_amount",
            *eyes_light_intensity_amount,
        );
        push_float_attr(&mut m, "Ladybag_amount", *ladybag_amount);
        push_float_attr(&mut m, "Lineart_amount", *lineart_amount);
        push_float_attr(&mut m, "Melting_glow_amount", *melting_glow_amount);
        push_float_attr(&mut m, "Pixel_amount", *pixel_amount);
        push_float_attr(&mut m, "Rain_amount", *rain_amount);
        push_float_attr(&mut m, "Smoke_amount", *smoke_amount);
        push_float_attr(
            &mut m,
            "Soap_bubble_intensity_amount",
            *soap_bubble_intensity_amount,
        );
        push_float_attr(
            &mut m,
            "Soap_bubble_roughness_amount",
            *soap_bubble_roughness_amount,
        );
        push_float_attr(&mut m, "Spring_amount", *spring_amount);
        push_float_attr(&mut m, "Underwater_fog_amount", *underwater_fog_amount);
        push_float_attr(&mut m, "Xray_body_amount", *xray_body_amount);
        push_float_attr(
            &mut m,
            "Xray_skeleton_particles_amount",
            *xray_skeleton_particles_amount,
        );
        push_float_attr(
            &mut m,
            "background_color_random_hue",
            *background_color_random_hue,
        );
        push_float_attr(
            &mut m,
            "background_underwater_color_hue",
            *background_underwater_color_hue,
        );
        push_float_attr(&mut m, "dress_color_hue", *dress_color_hue);
        push_float_attr(&mut m, "eye_color_random_hue", *eye_color_random_hue);
        push_float_attr(&mut m, "random_value", *random_value);

        push_int_attr(&mut m, "wedgeindex", *wedgeindex);

        push_float_attr(&mut m, "Render_noise_threshold", *render_noise_threshold);
        push_int_attr(&mut m, "Render_resolution", *render_resolution);

        /*
        push_int_attr(&mut m, "split", DEFAULT_SPLIT);
        push_int_attr(&mut m, "wedgecount", DEFAULT_WEDGECOUNT);
        push_int_attr(&mut m, "wedgenum", DEFAULT_WEDGENUM);
        push_int_attr(&mut m, "wedgetotal", DEFAULT_WEDGETOTAL);

        m.insert(
            "wedgeattribs".to_owned(),
            serde_json::to_value(&DEFAULT_WEDGEATTRIBS).unwrap(),
        );
        */

        Ok(m.into())
    }

    pub fn correction(&mut self) {
        fn correct_percent_value(v: &mut f64) {
            if *v < 0.0 {
                *v = 0.0;
            }
            if *v > 100.0 {
                *v = 100.0;
            }
        }
        correct_percent_value(&mut self.butterfly_amount);
        correct_percent_value(&mut self.disintegration_amount);
        correct_percent_value(&mut self.melt_amount);
        correct_percent_value(&mut self.fall_amount);
        correct_percent_value(&mut self.firefly_amount);
        correct_percent_value(&mut self.frozen_amount);
        correct_percent_value(&mut self.fungi_amount);
        correct_percent_value(&mut self.gold_silver_amount);
        correct_percent_value(&mut self.grow_flower_amount);
        correct_percent_value(&mut self.hologram_amount);
        correct_percent_value(&mut self.eyes_light_intensity_amount);
        correct_percent_value(&mut self.ladybag_amount);
        correct_percent_value(&mut self.lineart_amount);
        correct_percent_value(&mut self.melting_glow_amount);
        correct_percent_value(&mut self.pixel_amount);
        correct_percent_value(&mut self.rain_amount);
        correct_percent_value(&mut self.smoke_amount);
        correct_percent_value(&mut self.soap_bubble_intensity_amount);
        correct_percent_value(&mut self.soap_bubble_roughness_amount);
        correct_percent_value(&mut self.spring_amount);
        correct_percent_value(&mut self.underwater_fog_amount);
        correct_percent_value(&mut self.xray_body_amount);
        correct_percent_value(&mut self.xray_skeleton_particles_amount);

        fn correct_hue_value(v: &mut f64) {
            *v %= 360.0;
            if *v < 0.0 {
                *v += 360.0;
            }
        }
        correct_hue_value(&mut self.background_color_random_hue);
        correct_hue_value(&mut self.background_underwater_color_hue);
        correct_hue_value(&mut self.dress_color_hue);
        correct_hue_value(&mut self.eye_color_random_hue);
    }
}

macro_rules! impl_try_from_u32 {
    ($t:ident) => {
        impl std::convert::TryFrom<u32> for $t {
            type Error = FromPDGError;
            fn try_from(value: u32) -> Result<Self, Self::Error> {
                Self::from_repr(value)
                    .ok_or_else(|| unknown_variant(std::any::type_name::<Self>(), value))
            }
        }
    };
}

#[derive(
    strum::FromRepr,
    strum::EnumProperty,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
    Hash,
)]
#[repr(u32)]
pub enum BodyType {
    #[strum(props(PDGName = "Spacesuit"))]
    #[strum(props(MetaplexName = "Spacesuit"))]
    #[strum(props(weight = "20"))]
    #[default]
    Spacesuit = 0,
    #[strum(props(PDGName = "Sci Fi Police"))]
    #[strum(props(MetaplexName = "Sci-Fi Police"))]
    #[strum(props(weight = "15"))]
    SciFiPolice = 1,
    #[strum(props(PDGName = "Diver"))]
    #[strum(props(MetaplexName = "Diver"))]
    #[strum(props(weight = "20"))]
    Diver = 2,
    #[strum(props(PDGName = "Cyborg"))]
    #[strum(props(MetaplexName = "Cyborg"))]
    #[strum(props(weight = "5"))]
    Cyborg = 3,
    #[strum(props(PDGName = "Sci Fi sport woman"))]
    #[strum(props(MetaplexName = "Sci-Fi Sport Woman"))]
    #[strum(props(weight = "20"))]
    SciFiSportWoman = 4,
    #[strum(props(PDGName = "Sci Fi Exo costume"))]
    #[strum(props(MetaplexName = "Sci-Fi Exo Costume"))]
    #[strum(props(weight = "20"))]
    SciFiExoCostume = 5,
}

impl_try_from_u32!(BodyType);

impl BodyType {
    fn seed() -> Self {
        let mut rng = rand::thread_rng();
        let variants = BodyType::iter().collect::<Vec<_>>();
        let weights = variants
            .iter()
            .map(|v| {
                v.get_str("weight")
                    .unwrap_or("0")
                    .parse::<u32>()
                    .unwrap_or(0)
            })
            .collect::<Vec<_>>();
        let dist = WeightedIndex::new(weights).unwrap();

        variants[dist.sample(&mut rng)]
    }
}

#[derive(
    strum::FromRepr,
    strum::EnumProperty,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
    Hash,
)]
#[repr(u32)]
pub enum HelmetType {
    #[strum(props(PDGName = "Spacesuit"))]
    #[strum(props(MetaplexName = "Spacesuit"))]
    #[default]
    Spacesuit = 0,
    #[strum(props(PDGName = "Diver"))]
    #[strum(props(MetaplexName = "Diver"))]
    Diver = 1,
    #[strum(props(PDGName = "Diving Old"))]
    #[strum(props(MetaplexName = "Diving Old"))]
    DivingOld = 2,
    #[strum(props(PDGName = "Cyborg"))]
    #[strum(props(MetaplexName = "Cyborg"))]
    Cyborg = 3,
    #[strum(props(PDGName = "Pilot"))]
    #[strum(props(MetaplexName = "Pilot"))]
    Pilot = 4,
    #[strum(props(PDGName = "Pilot Old"))]
    #[strum(props(MetaplexName = "Pilot Old"))]
    PilotOld = 5,
    #[strum(props(PDGName = "Steam punk"))]
    #[strum(props(MetaplexName = "SteamPunk"))]
    Steampunk = 6,
    #[strum(props(PDGName = "Knight1"))]
    #[strum(props(MetaplexName = "Knight 1"))]
    Knight1 = 7,
    #[strum(props(PDGName = "Knight2"))]
    #[strum(props(MetaplexName = "Knight 2"))]
    Knight2 = 8,
    #[strum(props(PDGName = "Space mercury"))]
    #[strum(props(MetaplexName = "Space Mercury"))]
    SpaceMercury = 9,
    #[strum(props(PDGName = "Space soviet"))]
    #[strum(props(MetaplexName = "Space Soviet"))]
    SpaceSoviet = 10,
    #[strum(props(PDGName = "Sci_Fi_sport_woman"))]
    #[strum(props(MetaplexName = "Sci-Fi Sport Woman"))]
    SciFiSportWoman = 11,
    #[strum(props(PDGName = "Iron centaur"))]
    #[strum(props(MetaplexName = "Iron Centaur"))]
    IronCentaur = 12,
    #[strum(props(PDGName = "Sci Fi Exo costume"))]
    #[strum(props(MetaplexName = "Sci-Fi Exo Costume"))]
    SciFiExoCostume = 13,
    #[strum(props(PDGName = "Ghouls"))]
    #[strum(props(MetaplexName = "Ghouls"))]
    Ghouls = 14,
    #[strum(props(PDGName = "Gladiator"))]
    #[strum(props(MetaplexName = "Gladiator"))]
    Gladiator = 15,
    #[strum(props(PDGName = "Sci Fi racer helmet"))]
    #[strum(props(MetaplexName = "Sci-Fi Racer Helmet"))]
    SciFiRacerHelmet = 16,
    #[strum(props(PDGName = "Samurai"))]
    #[strum(props(MetaplexName = "Samurai"))]
    Samurai = 17,
}

impl_try_from_u32!(HelmetType);

impl HelmetType {
    fn seed() -> Self {
        let mut rng = rand::thread_rng();
        let variants = HelmetType::iter().collect::<Vec<_>>();
        let dist = Uniform::new(0, variants.len());

        variants[dist.sample(&mut rng)]
    }
}

#[derive(
    strum::FromRepr,
    strum::EnumProperty,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Display,
    Default,
    Hash,
)]
#[repr(u32)]
pub enum Pose {
    #[strum(props(PDGName = "Surprised"))]
    #[strum(props(MetaplexName = "Susprised"))]
    Surprised = 0,
    #[strum(props(PDGName = "Really?"))]
    #[strum(props(MetaplexName = "Really?"))]
    Really = 1,
    #[strum(props(PDGName = "Overconfident"))]
    #[strum(props(MetaplexName = "Overconfident"))]
    Overconfident = 2,
    #[strum(props(PDGName = "Instigate"))]
    #[strum(props(MetaplexName = "Instigate"))]
    Instigate = 3,
    #[strum(props(PDGName = "Inspired"))]
    #[strum(props(MetaplexName = "Inspired"))]
    Inspired = 4,
    #[strum(props(PDGName = "Listening"))]
    #[strum(props(MetaplexName = "Listening"))]
    Listening = 5,
    #[strum(props(PDGName = "Side eye"))]
    #[strum(props(MetaplexName = "Side Eye"))]
    SideEye = 6,
    #[strum(props(PDGName = "Mugshot"))]
    #[strum(props(MetaplexName = "Mugshot"))]
    Mugshot = 7,
    #[strum(props(PDGName = "OG (original NFT)"))]
    #[strum(props(MetaplexName = "OG (Original NFT)"))]
    #[default]
    Original = 8,
    #[strum(props(PDGName = "Suspicious"))]
    #[strum(props(MetaplexName = "Suspicious"))]
    Suspicious = 9,
    #[strum(props(PDGName = "Thinking"))]
    #[strum(props(MetaplexName = "Thinking"))]
    Thinking = 10,
    #[strum(props(PDGName = "Busy"))]
    #[strum(props(MetaplexName = "Busy"))]
    Busy = 11,
    #[strum(props(PDGName = "Ready"))]
    #[strum(props(MetaplexName = "Ready"))]
    Ready = 12,
    #[strum(props(PDGName = "Stare"))]
    #[strum(props(MetaplexName = "Stare"))]
    Stare = 13,
    #[strum(props(PDGName = "Introspection"))]
    #[strum(props(MetaplexName = "Introspection"))]
    Introspection = 14,
    #[strum(props(PDGName = "Look up left"))]
    #[strum(props(MetaplexName = "Look Up Left"))]
    LookUpLeft = 15,
    #[strum(props(PDGName = "Look up"))]
    #[strum(props(MetaplexName = "Look Up"))]
    LookUp = 16,
    #[strum(props(PDGName = "Look up right"))]
    #[strum(props(MetaplexName = "Look Up Right"))]
    LookUpRight = 17,
    #[strum(props(PDGName = "Look left"))]
    #[strum(props(MetaplexName = "Look Left"))]
    LookLeft = 18,
    #[strum(props(PDGName = "Default"))]
    #[strum(props(MetaplexName = "Default"))]
    Default = 19,
    #[strum(props(PDGName = "Look right"))]
    #[strum(props(MetaplexName = "Look Right"))]
    LookRight = 20,
    #[strum(props(PDGName = "Look down left"))]
    #[strum(props(MetaplexName = "Look Down Left"))]
    LookDownLeft = 21,
    #[strum(props(PDGName = "Look down"))]
    #[strum(props(MetaplexName = "Look Down"))]
    LookDown = 22,
    #[strum(props(PDGName = "Look down right"))]
    #[strum(props(MetaplexName = "Look Down Right"))]
    LookDownRight = 23,
}

impl_try_from_u32!(Pose);

impl Pose {
    fn seed() -> Self {
        let mut rng = rand::thread_rng();
        let variants = Pose::iter().collect::<Vec<_>>();
        let dist = Uniform::new(0, variants.len());

        variants[dist.sample(&mut rng)]
    }
}

#[derive(
    strum::FromRepr,
    strum::EnumProperty,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
)]
#[repr(u32)]
pub enum HelmetLight {
    #[strum(props(PDGName = "No"))]
    #[strum(props(MetaplexName = "No"))]
    #[strum(props(weight = "50"))]
    #[default]
    Off = 0,
    #[strum(props(PDGName = "Dots"))]
    #[strum(props(MetaplexName = "Dots"))]
    #[strum(props(weight = "25"))]
    Dots = 1,
    #[strum(props(PDGName = "Glowing eyes"))]
    #[strum(props(MetaplexName = "Glowing Eyes"))]
    #[strum(props(weight = "15"))]
    GlowingEyes = 2,
    #[strum(props(PDGName = "Solana"))]
    #[strum(props(MetaplexName = "Solana"))]
    #[strum(props(weight = "10"))]
    Solana = 3,
}

impl_try_from_u32!(HelmetLight);

impl HelmetLight {
    fn seed() -> Self {
        let mut rng = rand::thread_rng();
        let variants = HelmetLight::iter().collect::<Vec<_>>();
        let weights = variants
            .iter()
            .map(|v| {
                v.get_str("weight")
                    .unwrap_or("0")
                    .parse::<u32>()
                    .unwrap_or(0)
            })
            .collect::<Vec<_>>();
        let dist = WeightedIndex::new(weights).unwrap();

        variants[dist.sample(&mut rng)]
    }
}

#[derive(
    strum::FromRepr,
    strum::EnumProperty,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
    Display,
    Hash,
)]
#[repr(u32)]
pub enum Fx0 {
    #[strum(props(PDGName = "No"))]
    #[strum(props(MetaplexName = "No"))]
    #[strum(props(weight = "40"))]
    #[default]
    No = 0,
    #[strum(props(PDGName = "Marble"))]
    #[strum(props(MetaplexName = "Marble"))]
    #[strum(props(weight = "10"))]
    Marble = 1,
    #[strum(props(PDGName = "Pixel"))]
    #[strum(props(MetaplexName = "Pixel"))]
    #[strum(props(weight = "10"))]
    Pixel = 2,
    #[strum(props(PDGName = "Lineart base"))]
    #[strum(props(MetaplexName = "Lineart Base"))]
    #[strum(props(weight = "10"))]
    LineartBase = 3,
    #[strum(props(PDGName = "Wood"))]
    #[strum(props(MetaplexName = "Wood"))]
    #[strum(props(weight = "10"))]
    Wood = 4,
    #[strum(props(PDGName = "Hologram"))]
    #[strum(props(MetaplexName = "Hologram"))]
    #[strum(props(weight = "5"))]
    Hologram = 5,
    #[strum(props(PDGName = "Xray"))]
    #[strum(props(MetaplexName = "X-ray"))]
    #[strum(props(weight = "5"))]
    Xray = 6,
    #[strum(props(PDGName = "Soap bubble"))]
    #[strum(props(MetaplexName = "Soap Bubble"))]
    #[strum(props(weight = "10"))]
    SoapBubble = 7,
}

impl_try_from_u32!(Fx0);

impl Fx0 {
    fn seed() -> Self {
        let mut rng = rand::thread_rng();
        let variants = Fx0::iter().collect::<Vec<_>>();
        let weights = variants
            .iter()
            .map(|v| {
                v.get_str("weight")
                    .unwrap_or("0")
                    .parse::<u32>()
                    .unwrap_or(0)
            })
            .collect::<Vec<_>>();
        let dist = WeightedIndex::new(weights).unwrap();

        variants[dist.sample(&mut rng)]
    }
}

#[derive(
    strum::FromRepr,
    strum::EnumProperty,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
    Display,
    Hash,
)]
#[repr(u32)]
pub enum Fx1 {
    #[strum(props(PDGName = "No"))]
    #[strum(props(MetaplexName = "No"))]
    #[strum(props(weight = "80"))]
    #[default]
    No = 0,
    #[strum(props(PDGName = "Melted"))]
    #[strum(props(MetaplexName = "Melted"))]
    #[strum(props(weight = "5"))]
    Melted = 1,
    #[strum(props(PDGName = "Disintegration"))]
    #[strum(props(MetaplexName = "Disintegration"))]
    #[strum(props(weight = "15"))]
    Disintegration = 2,
}

impl_try_from_u32!(Fx1);

impl Fx1 {
    fn seed() -> Self {
        let mut rng = rand::thread_rng();
        let variants = Fx1::iter().collect::<Vec<_>>();
        let weights = variants
            .iter()
            .map(|v| {
                v.get_str("weight")
                    .unwrap_or("0")
                    .parse::<u32>()
                    .unwrap_or(0)
            })
            .collect::<Vec<_>>();
        let dist = WeightedIndex::new(weights).unwrap();

        variants[dist.sample(&mut rng)]
    }
}

#[derive(
    strum::FromRepr,
    strum::EnumProperty,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
    Display,
    Hash,
)]
#[repr(u32)]
pub enum Fx1a {
    #[strum(props(PDGName = "No"))]
    #[default]
    #[strum(props(weight = "0"))]
    No = 0,
    #[strum(props(PDGName = "Lineart minimalistic"))]
    #[strum(props(weight = "50"))]
    LineartMinimalistic = 1,
    #[strum(props(PDGName = "Lineart Heavy"))]
    #[strum(props(weight = "50"))]
    LineartHeavy = 2,
}

impl_try_from_u32!(Fx1a);

impl Fx1a {
    fn seed() -> Self {
        let mut rng = rand::thread_rng();
        let variants = Fx1a::iter().collect::<Vec<_>>();
        let dist = Uniform::new(0, variants.len());

        variants[dist.sample(&mut rng)]
    }
}

#[derive(
    strum::FromRepr,
    strum::EnumProperty,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
    Display,
    Hash,
)]
#[repr(u32)]
pub enum Fx2 {
    #[strum(props(PDGName = "No"))]
    #[strum(props(MetaplexName = "No"))]
    #[strum(props(weight = "45"))]
    #[default]
    No = 0,
    #[strum(props(PDGName = "Butterflies"))]
    #[strum(props(MetaplexName = "Butterfly"))]
    #[strum(props(weight = "10"))]
    Butterflies = 1,
    #[strum(props(PDGName = "Underwater"))]
    #[strum(props(MetaplexName = "Underwater"))]
    #[strum(props(weight = "5"))]
    Underwater = 2,
    #[strum(props(PDGName = "Fireflyies"))]
    #[strum(props(MetaplexName = "Firefly"))]
    #[strum(props(weight = "10"))]
    Fireflyies = 3,
    #[strum(props(PDGName = "Fall"))]
    #[strum(props(MetaplexName = "Fall"))]
    #[strum(props(weight = "10"))]
    Fall = 4,
    #[strum(props(PDGName = "Ladybag"))]
    #[strum(props(MetaplexName = "Ladybug"))]
    #[strum(props(weight = "10"))]
    Ladybag = 5,
    #[strum(props(PDGName = "Spring"))]
    #[strum(props(MetaplexName = "Spring"))]
    #[strum(props(weight = "10"))]
    Spring = 6,
}

impl_try_from_u32!(Fx2);

impl Fx2 {
    fn seed() -> Self {
        let mut rng = rand::thread_rng();
        let variants = Fx2::iter().collect::<Vec<_>>();
        let weights = variants
            .iter()
            .map(|v| {
                v.get_str("weight")
                    .unwrap_or("0")
                    .parse::<u32>()
                    .unwrap_or(0)
            })
            .collect::<Vec<_>>();
        let dist = WeightedIndex::new(weights).unwrap();

        variants[dist.sample(&mut rng)]
    }
}

#[derive(
    strum::FromRepr,
    strum::EnumProperty,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
    Display,
    Hash,
)]
#[repr(u32)]
pub enum Fx3 {
    #[strum(props(PDGName = "No"))]
    #[strum(props(MetaplexName = "No"))]
    #[default]
    No = 0,
    #[strum(props(PDGName = "Smoke"))]
    #[strum(props(MetaplexName = "Yes"))]
    Smoke = 1,
}

impl_try_from_u32!(Fx3);

#[derive(
    strum::FromRepr,
    strum::EnumProperty,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
    Display,
    Hash,
)]
#[repr(u32)]
pub enum Fx4 {
    #[strum(props(PDGName = "No"))]
    #[strum(props(MetaplexName = "No"))]
    #[strum(props(weight = "75"))]
    #[default]
    No = 0,
    #[strum(props(PDGName = "Frozen"))]
    #[strum(props(MetaplexName = "Frozen"))]
    #[strum(props(weight = "10"))]
    Frozen = 1,
    #[strum(props(PDGName = "Rain"))]
    #[strum(props(MetaplexName = "Rain"))]
    #[strum(props(weight = "15"))]
    Rain = 2,
}

impl_try_from_u32!(Fx4);

impl Fx4 {
    fn seed() -> Self {
        let mut rng = rand::thread_rng();
        let variants = Fx4::iter().collect::<Vec<_>>();
        let weights = variants
            .iter()
            .map(|v| {
                v.get_str("weight")
                    .unwrap_or("0")
                    .parse::<u32>()
                    .unwrap_or(0)
            })
            .collect::<Vec<_>>();
        let dist = WeightedIndex::new(weights).unwrap();

        variants[dist.sample(&mut rng)]
    }
}

#[derive(
    strum::FromRepr,
    strum::EnumProperty,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
    Display,
    Hash,
)]
#[repr(u32)]
pub enum Fx5 {
    #[strum(props(PDGName = "No"))]
    #[strum(props(MetaplexName = "No"))]
    #[strum(props(weight = "70"))]
    #[default]
    No = 0,
    #[strum(props(PDGName = "Fungi"))]
    #[strum(props(MetaplexName = "Fungi"))]
    #[strum(props(weight = "15"))]
    Fungi = 1,
    #[strum(props(PDGName = "GrowFlower"))]
    #[strum(props(MetaplexName = "Flower"))]
    #[strum(props(weight = "15"))]
    GrowFlower = 2,
}

impl_try_from_u32!(Fx5);

impl Fx5 {
    fn seed() -> Self {
        let mut rng = rand::thread_rng();
        let variants = Fx5::iter().collect::<Vec<_>>();
        let weights = variants
            .iter()
            .map(|v| {
                v.get_str("weight")
                    .unwrap_or("0")
                    .parse::<u32>()
                    .unwrap_or(0)
            })
            .collect::<Vec<_>>();
        let dist = WeightedIndex::new(weights).unwrap();

        variants[dist.sample(&mut rng)]
    }
}

#[derive(
    strum::FromRepr,
    strum::EnumProperty,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
    Display,
    Hash,
)]
#[repr(u32)]
pub enum Fx6 {
    #[strum(props(PDGName = "No"))]
    #[strum(props(MetaplexName = "No"))]
    #[strum(props(weight = "50"))]
    #[default]
    No = 0,
    #[strum(props(PDGName = "Gold"))]
    #[strum(props(MetaplexName = "Gold"))]
    #[strum(props(weight = "5"))]
    Gold = 1,
    #[strum(props(PDGName = "Silver"))]
    #[strum(props(MetaplexName = "Silver"))]
    #[strum(props(weight = "10"))]
    Silver = 2,
    #[strum(props(PDGName = "Rose Gold"))]
    #[strum(props(MetaplexName = "Rose Gold"))]
    #[strum(props(weight = "5"))]
    RoseGold = 3,
    #[strum(props(PDGName = "Copper"))]
    #[strum(props(MetaplexName = "Copper"))]
    #[strum(props(weight = "15"))]
    Copper = 4,
    #[strum(props(PDGName = "Bronze"))]
    #[strum(props(MetaplexName = "Bronze"))]
    #[strum(props(weight = "15"))]
    Bronze = 5,
}

impl_try_from_u32!(Fx6);

impl Fx6 {
    fn seed() -> Self {
        let mut rng = rand::thread_rng();
        let variants = Fx6::iter().collect::<Vec<_>>();
        let weights = variants
            .iter()
            .map(|v| {
                v.get_str("weight")
                    .unwrap_or("0")
                    .parse::<u32>()
                    .unwrap_or(0)
            })
            .collect::<Vec<_>>();
        let dist = WeightedIndex::new(weights).unwrap();

        variants[dist.sample(&mut rng)]
    }
}

#[derive(
    strum::FromRepr,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
)]
#[repr(u32)]
pub enum Fx0BodyOff {
    #[strum(props(PDGName = "No"))]
    #[strum(props(MetaplexName = "No Body"))]
    #[strum(props(weight = "50"))]
    #[default]
    No = 0,
    #[strum(props(PDGName = "Visible"))]
    #[strum(props(MetaplexName = "Visible"))]
    #[strum(props(weight = "5"))]
    On = 1,
}

impl_try_from_u32!(Fx0BodyOff);

#[derive(
    strum::FromRepr,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
)]
#[repr(u32)]
pub enum Fx0BodyOffGlass {
    #[strum(props(PDGName = "No"))]
    #[strum(props(MetaplexName = "No Glass"))]
    #[strum(props(weight = "50"))]
    #[default]
    No = 0,
    #[strum(props(PDGName = "On"))]
    #[strum(props(MetaplexName = "Visible"))]
    #[strum(props(weight = "5"))]
    On = 1,
}

impl_try_from_u32!(Fx0BodyOffGlass);

#[derive(
    strum::FromRepr,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
)]
#[repr(u32)]
pub enum BodyMaterialVariations {
    #[default]
    StandardTextures = 0,
    Stripes = 1,
    Dots = 2,
    Felt = 3,
}

impl_try_from_u32!(BodyMaterialVariations);

impl BodyMaterialVariations {
    fn seed() -> Self {
        let mut rng = rand::thread_rng();
        let variants = BodyMaterialVariations::iter().collect::<Vec<_>>();
        let dist = Uniform::new(0, variants.len());

        variants[dist.sample(&mut rng)]
    }
}

#[derive(
    strum::FromRepr,
    strum::EnumProperty,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
    Display,
    Hash,
)]
#[repr(u32)]
pub enum MarbleVariation {
    #[strum(props(MetaplexName = "Zero"))]
    #[strum(props(weight = "50"))]
    #[default]
    Zero = 0,
    #[strum(props(MetaplexName = "One"))]
    #[strum(props(weight = "5"))]
    One = 1,
    #[strum(props(MetaplexName = "Two"))]
    #[strum(props(weight = "10"))]
    Two = 2,
    #[strum(props(MetaplexName = "Three"))]
    #[strum(props(weight = "5"))]
    Three = 3,
    #[strum(props(MetaplexName = "Four"))]
    #[strum(props(weight = "15"))]
    Four = 4,
    #[strum(props(MetaplexName = "Five"))]
    #[strum(props(weight = "15"))]
    Five = 5,
    #[strum(props(MetaplexName = "Six"))]
    #[strum(props(weight = "15"))]
    Six = 6,
    #[strum(props(MetaplexName = "Seven"))]
    #[strum(props(weight = "15"))]
    Seven = 7,
    #[strum(props(MetaplexName = "Eight"))]
    #[strum(props(weight = "15"))]
    Eight = 8,

}

impl_try_from_u32!(MarbleVariation);

impl MarbleVariation {
    fn seed() -> Self {
        let mut rng = rand::thread_rng();
        let variants = MarbleVariation::iter().collect::<Vec<_>>();
        let weights = variants
            .iter()
            .map(|v| {
                v.get_str("weight")
                    .unwrap_or("0")
                    .parse::<u32>()
                    .unwrap_or(0)
            })
            .collect::<Vec<_>>();
        let dist = WeightedIndex::new(weights).unwrap();

        variants[dist.sample(&mut rng)]
    }
}

#[derive(
    strum::FromRepr,
    strum::EnumProperty,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
    Display,
    Hash,
)]
#[repr(u32)]
pub enum WoodVariation {
    #[strum(props(MetaplexName = "Zero"))]
    #[strum(props(weight = "50"))]
    #[default]
    Zero = 0,
    #[strum(props(MetaplexName = "One"))]
    #[strum(props(weight = "5"))]
    One = 1,
    #[strum(props(MetaplexName = "Two"))]
    #[strum(props(weight = "10"))]
    Two = 2,
    #[strum(props(MetaplexName = "Three"))]
    #[strum(props(weight = "5"))]
    Three = 3,
    #[strum(props(MetaplexName = "Four"))]
    #[strum(props(weight = "15"))]
    Four = 4,
    #[strum(props(MetaplexName = "Five"))]
    #[strum(props(weight = "15"))]
    Five = 5,
    #[strum(props(MetaplexName = "Six"))]
    #[strum(props(weight = "15"))]
    Six = 6,
    #[strum(props(MetaplexName = "Seven"))]
    #[strum(props(weight = "15"))]
    Seven = 7,
    #[strum(props(MetaplexName = "Eight"))]
    #[strum(props(weight = "15"))]
    Eight = 8,
}

impl_try_from_u32!(WoodVariation);

impl WoodVariation {
    fn seed() -> Self {
        let mut rng = rand::thread_rng();
        let variants = WoodVariation::iter().collect::<Vec<_>>();
        let weights = variants
            .iter()
            .map(|v| {
                v.get_str("weight")
                    .unwrap_or("0")
                    .parse::<u32>()
                    .unwrap_or(0)
            })
            .collect::<Vec<_>>();
        let dist = WeightedIndex::new(weights).unwrap();

        variants[dist.sample(&mut rng)]
    }
}

#[derive(
    strum::FromRepr,
    strum::EnumProperty,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
    Display,
    Hash,
)]
#[repr(u32)]
pub enum GlowingLogo {
    #[strum(props(weight = "90"))]
    #[default]
    No = 0,
    #[strum(props(weight = "10"))]
    Yes = 1,
}

impl_try_from_u32!(GlowingLogo);

impl GlowingLogo {
    fn seed() -> Self {
        let mut rng = rand::thread_rng();
        let variants = GlowingLogo::iter().collect::<Vec<_>>();
        let weights = variants
            .iter()
            .map(|v| {
                v.get_str("weight")
                    .unwrap_or("0")
                    .parse::<u32>()
                    .unwrap_or(0)
            })
            .collect::<Vec<_>>();
        let dist = WeightedIndex::new(weights).unwrap();

        variants[dist.sample(&mut rng)]
    }
}

#[derive(
    strum::FromRepr,
    strum::EnumProperty,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
    Display,
    Hash,
)]
#[repr(u32)]
pub enum FxJellyfish {
    #[strum(props(PDGName = "No"))]
    #[strum(props(weight = "80"))]
    #[default]
    No = 0,
    #[strum(props(PDGName = "Yes"))]
    #[strum(props(weight = "20"))]
    Yes = 1,
}

impl_try_from_u32!(FxJellyfish);

impl FxJellyfish {
    fn seed() -> Self {
        let mut rng = rand::thread_rng();
        let variants = FxJellyfish::iter().collect::<Vec<_>>();
        let weights = variants
            .iter()
            .map(|v| {
                v.get_str("weight")
                    .unwrap_or("0")
                    .parse::<u32>()
                    .unwrap_or(0)
            })
            .collect::<Vec<_>>();
        let dist = WeightedIndex::new(weights).unwrap();

        variants[dist.sample(&mut rng)]
    }
}

#[derive(
    strum::FromRepr,
    strum::EnumIter,
    strum::EnumProperty,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
    Display,
    Hash,
)]
#[repr(u32)]
pub enum FxLineartHelper {
    #[default]
    #[strum(props(weight = "50"))]
    Zero = 0,
    #[strum(props(weight = "50"))]
    One = 1,
}

impl_try_from_u32!(FxLineartHelper);

impl FxLineartHelper {
    fn seed() -> Self {
        let mut rng = rand::thread_rng();
        let variants = FxLineartHelper::iter().collect::<Vec<_>>();
        let weights = variants
            .iter()
            .map(|v| {
                v.get_str("weight")
                    .unwrap_or("50")
                    .parse::<u32>()
                    .unwrap_or(50)
            })
            .collect::<Vec<_>>();
        let dist = WeightedIndex::new(weights).unwrap();

        variants[dist.sample(&mut rng)]
    }
}

#[derive(
    strum::FromRepr,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
)]
#[repr(u32)]
pub enum EnvLight {
    #[default]
    Day = 0,
    Night = 1,
    Underwater = 2,
    UnderwaterHologram = 3,
}

impl_try_from_u32!(EnvLight);

impl EnvLight {
    pub fn day_or_night() -> Self {
        let mut rng = rand::thread_rng();
        let variants = [EnvLight::Day, EnvLight::Night];
        let weights = vec![50, 50];
        let dist = WeightedIndex::new(weights).unwrap();

        variants[dist.sample(&mut rng)]
    }
}

#[derive(
    strum::FromRepr,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
)]
#[repr(u32)]
pub enum EnvReflection {
    #[default]
    Off = 0,
    On = 1,
}

impl_try_from_u32!(EnvReflection);

#[derive(
    strum::FromRepr,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Default,
)]
#[repr(u32)]
pub enum LightReflectionMult {
    #[default]
    One = 1,
    Two = 2,
}

impl_try_from_u32!(LightReflectionMult);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_metadata() {
        let mut json =
            serde_json::from_str::<serde_json::Value>(include_str!("tests/123.json")).unwrap();
        let meta = RenderParams::from_pdg_metadata(&mut json, true).unwrap();
        println!("{:#}", json);
        println!(
            "{:#?}",
            json.as_object().unwrap().keys().collect::<Vec<_>>()
        );
        dbg!(&meta);
        let mut pdg = meta.to_pdg_metadata(true).unwrap();
        println!("{:#}", pdg);
        let meta1 = RenderParams::from_pdg_metadata(&mut pdg, true).unwrap();
        assert_eq!(meta, meta1);
        assert_eq!(
            pdg.as_object().unwrap().keys().next().unwrap(),
            "wedgeattribs"
        );
    }
}
