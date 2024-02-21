use super::{
    metaplex::COLOR_NAMES, BodyMaterialVariations, BodyType, EnumRandExt, EnvLight, Fx0, Fx1, Fx1a,
    Fx2, Fx3, Fx4, Fx5, Fx6, FxJellyfish, FxLineartHelper, GlowingLogo, HelmetLight, HelmetType,
    LightReflectionMult, MarbleVariation, Pose, RenderParams, WoodVariation,
};
use indexmap::IndexSet;
use rand::{seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

pub fn random_hue<R: rand::Rng + ?Sized>(rng: &mut R) -> f64 {
    const NUM_COLORS: usize = COLOR_NAMES.len();
    rng.gen_range(0..NUM_COLORS) as f64 * (360.0 / NUM_COLORS as f64)
}

/// Effects that an NFT can gain
#[derive(
    derive_more::From,
    strum::EnumProperty,
    strum::EnumIter,
    Debug,
    Clone,
    Copy,
    Eq,
    PartialEq,
    Serialize,
    Deserialize,
    Hash,
)]
#[serde(tag = "type", content = "value")]
pub enum Effect {
    #[strum(props(EffectType = "Pose"))]
    #[from]
    Pose(Pose),
    #[strum(props(EffectType = "Fx0"))]
    #[from]
    Fx0(Fx0),
    #[strum(props(EffectType = "Fx1"))]
    #[from]
    Fx1(Fx1),
    #[strum(props(EffectType = "Fx2"))]
    #[from]
    Fx2(Fx2),
    #[strum(props(EffectType = "Fx3"))]
    #[from]
    Fx3(Fx3),
    #[strum(props(EffectType = "Fx4"))]
    #[from]
    Fx4(Fx4),
    #[strum(props(EffectType = "Fx5"))]
    #[from]
    Fx5(Fx5),
    #[strum(props(EffectType = "Fx6"))]
    #[from]
    Fx6(Fx6),
    #[strum(props(EffectType = "Fx1a"))]
    #[from]
    Fx1a(Fx1a),
    #[strum(props(EffectType = "FxJellyfish"))]
    #[from]
    FxJellyfish(FxJellyfish),
    #[strum(props(EffectType = "FxLineartHelper"))]
    #[from]
    FxLineartHelper(FxLineartHelper),
}

pub struct EffectsList {
    pub effects: IndexSet<Effect>,
}

impl From<IndexSet<Effect>> for EffectsList {
    fn from(effects: IndexSet<Effect>) -> Self {
        Self { effects }
    }
}

impl From<Vec<Effect>> for EffectsList {
    fn from(effects: Vec<Effect>) -> Self {
        Self {
            effects: effects.into_iter().collect(),
        }
    }
}

impl<const N: usize> From<[Effect; N]> for EffectsList {
    fn from(effects: [Effect; N]) -> Self {
        Self {
            effects: effects.into(),
        }
    }
}

impl From<RenderParams> for EffectsList {
    fn from(value: RenderParams) -> Self {
        let RenderParams {
            body_type: _,
            pose,
            helmet_type: _,
            helmet_light: _,
            fx0,
            fx1,
            fx1a,
            fx2,
            fx3,
            fx4,
            fx5,
            fx6,
            fx0_bodyoff: _,
            fx0_bodyoff_glass: _,
            body_material_variation: _,
            marble_variation: _,
            wood_variation: _,
            fx_jellifish,
            fx_lineart_helper,
            env_light: _,
            env_reflection: _,
            light_reflection_mult: _,
            glowing_logo: _,
            logo_hue: _,
            logo_name: _,
            butterfly_amount: _,
            disintegration_amount: _,
            melt_amount: _,
            fall_amount: _,
            firefly_amount: _,
            frozen_amount: _,
            fungi_amount: _,
            gold_silver_amount: _,
            grow_flower_amount: _,
            hologram_amount: _,
            eyes_light_intensity_amount: _,
            ladybag_amount: _,
            lineart_amount: _,
            melting_glow_amount: _,
            pixel_amount: _,
            rain_amount: _,
            smoke_amount: _,
            soap_bubble_intensity_amount: _,
            soap_bubble_roughness_amount: _,
            spring_amount: _,
            underwater_fog_amount: _,
            xray_body_amount: _,
            xray_skeleton_particles_amount: _,
            background_color_random_hue: _,
            background_underwater_color_hue: _,
            dress_color_hue: _,
            eye_color_random_hue: _,
            random_value: _,
            wedgeindex: _,
            render_noise_threshold: _,
            render_resolution: _,
            wedgeattribs: _,
        } = value;
        Self {
            effects: [
                pose.into(),
                fx0.into(),
                fx1.into(),
                fx2.into(),
                fx3.into(),
                fx4.into(),
                fx5.into(),
                fx6.into(),
                fx1a.into(),
                fx_jellifish.into(),
                fx_lineart_helper.into(),
            ]
            .into(),
        }
    }
}

impl EffectsList {
    pub fn effect_lottery<R: Rng + ?Sized>(
        &self,
        mut choose_from: Vec<Effect>,
        rng: &mut R,
    ) -> Option<Effect> {
        choose_from.retain(|e| !self.effects.contains(e));
        choose_from.choose(rng).cloned()
    }

    pub fn push(&mut self, effect: Effect) -> bool {
        self.effects.insert(effect)
    }
}

impl Effect {
    pub fn all_effects() -> Vec<Effect> {
        let mut list = Vec::new();
        for e in Effect::iter() {
            match e {
                Effect::Pose(_) => list.extend(Pose::iter().map(Effect::from)),
                Effect::Fx0(_) => list.extend(Fx0::iter().map(Effect::from)),
                Effect::Fx1(_) => list.extend(Fx1::iter().map(Effect::from)),
                Effect::Fx2(_) => list.extend(Fx2::iter().map(Effect::from)),
                Effect::Fx3(_) => list.extend(Fx3::iter().map(Effect::from)),
                Effect::Fx4(_) => list.extend(Fx4::iter().map(Effect::from)),
                Effect::Fx5(_) => list.extend(Fx5::iter().map(Effect::from)),
                Effect::Fx6(_) => list.extend(Fx6::iter().map(Effect::from)),
                Effect::Fx1a(_) => list.extend(Fx1a::iter().map(Effect::from)),
                Effect::FxJellyfish(_) => list.extend(FxJellyfish::iter().map(Effect::from)),
                Effect::FxLineartHelper(_) => {
                    list.extend(FxLineartHelper::iter().map(Effect::from))
                }
            }
        }
        list
    }
}

impl RenderParams {
    pub fn add_effect(&mut self, effect: Effect) {
        match effect {
            Effect::Pose(x) => self.pose = x,
            Effect::Fx0(x) => self.fx0 = x,
            Effect::Fx1(x) => self.fx1 = x,
            Effect::Fx2(x) => self.fx2 = x,
            Effect::Fx3(x) => self.fx3 = x,
            Effect::Fx4(x) => self.fx4 = x,
            Effect::Fx5(x) => self.fx5 = x,
            Effect::Fx6(x) => self.fx6 = x,
            Effect::Fx1a(x) => self.fx1a = x,
            Effect::FxJellyfish(x) => self.fx_jellifish = x,
            Effect::FxLineartHelper(x) => self.fx_lineart_helper = x,
        }
    }

    pub fn generate_base<R: rand::Rng + ?Sized>(rng: &mut R) -> Self {
        let body_type = BodyType::choose(rng);
        let pose = Pose::choose(rng);
        let helmet_type = HelmetType::choose(rng);
        let helmet_light = HelmetLight::choose(rng);
        let fx0 = Fx0::choose(rng);

        Self {
            body_type,
            pose,
            helmet_type,
            helmet_light,
            fx0,
            ..<_>::default()
        }
        .adjust_base(rng)
        .generate_line_art(rng)
        .generate_fx(rng)
        .generate_underwater(rng)
        .generate_background_color(rng)
        .generate_dress_hue(rng)
        .generate_helmet_lights(rng)
        .generate_wedge(rng)
        .generate_body_material_variation(rng)
        .generate_marble_variation(rng)
        .generate_wood_variation(rng)
        .glowing_logo(rng)
        .generate_smoke(rng)
        .generate_random_value(rng)
    }

    pub fn adjust_base<R: rand::Rng + ?Sized>(mut self, rng: &mut R) -> Self {
        match self.fx0 {
            Fx0::Hologram => {
                self.env_light = EnvLight::Night;
                self.hologram_amount = rng.gen_range(25.0..=100.0)
            }
            Fx0::Xray => {
                self.env_light = EnvLight::day_or_night(rng);
                self.xray_skeleton_particles_amount = rng.gen_range(25.0..=100.0);
                self.xray_body_amount = rng.gen_range(25.0..=100.0);
            }
            Fx0::SoapBubble => {
                self.env_light = EnvLight::day_or_night(rng);
                self.soap_bubble_intensity_amount = rng.gen_range(25.0..=100.0);
                self.soap_bubble_roughness_amount = rng.gen_range(25.0..=100.0);
                self.light_reflection_mult = LightReflectionMult::Two;
            }
            Fx0::Pixel => {
                self.env_light = EnvLight::day_or_night(rng);
                self.pixel_amount = rng.gen::<f64>() * rng.gen_range(20.0..=40.0);
            }
            _ => {
                self.env_light = EnvLight::day_or_night(rng);
            }
        }
        self
    }

    pub fn generate_line_art<R: rand::Rng + ?Sized>(mut self, rng: &mut R) -> Self {
        match self.fx0 {
            Fx0::LineartBase => {
                let line_art = Fx1a::choose(rng);
                self.lineart_amount = rng.gen_range(0.0..100.0);

                self.fx1a = line_art;
                match line_art {
                    Fx1a::LineartMinimalistic => {
                        let fx_lineart_helper = FxLineartHelper::Zero;
                        self.fx_lineart_helper = fx_lineart_helper;
                    }
                    Fx1a::LineartHeavy => {
                        let fx_lineart_helper = FxLineartHelper::Zero;
                        self.fx_lineart_helper = fx_lineart_helper;
                        self.helmet_light = HelmetLight::some_lights(rng);
                    }
                    Fx1a::No => {
                        let fx_lineart_helper = FxLineartHelper::Zero;
                        self.fx_lineart_helper = fx_lineart_helper;
                    }
                }
            }
            _ => {
                let line_art = Fx1a::none_or_minimal(rng);
                self.fx1a = line_art;
                match line_art {
                    Fx1a::No => {
                        let fx_lineart_helper = FxLineartHelper::choose(rng);
                        self.fx_lineart_helper = fx_lineart_helper;
                    }
                    Fx1a::LineartMinimalistic => {
                        let fx_lineart_helper = FxLineartHelper::One;
                        self.fx_lineart_helper = fx_lineart_helper;
                    }
                    Fx1a::LineartHeavy => {}
                }
            }
        }
        self
    }

    pub fn generate_fx<R: rand::Rng + ?Sized>(mut self, rng: &mut R) -> Self {
        let fx1 = Fx1::choose(rng);

        self.fx1 = fx1;
        match fx1 {
            Fx1::No => {}
            Fx1::Melted => {
                self.melt_amount = rng.gen::<f64>() * 30.0;
                self.melting_glow_amount = 50.0;
            }
            Fx1::Disintegration => self.disintegration_amount = rng.gen::<f64>() * 30.0,
        }

        let fx2 = Fx2::choose(rng);
        self.fx2 = fx2;
        match fx2 {
            Fx2::No => {}
            Fx2::Butterflies => self.butterfly_amount = rng.gen::<f64>() * 30.0,
            Fx2::Underwater => {} // underwater is handled separately
            Fx2::Fireflyies => self.firefly_amount = rng.gen::<f64>() * 30.0,
            Fx2::Fall => self.fall_amount = rng.gen::<f64>() * 30.0,
            Fx2::Ladybag => self.ladybag_amount = rng.gen::<f64>() * 30.0,
            Fx2::Spring => self.spring_amount = rng.gen::<f64>() * 30.0,
        }

        let fx4 = Fx4::choose(rng);
        self.fx4 = fx4;
        match fx4 {
            Fx4::No => {}
            Fx4::Frozen => self.frozen_amount = rng.gen::<f64>() * 30.0,
            Fx4::Rain => self.rain_amount = rng.gen::<f64>() * 40.0,
        }

        let fx5 = Fx5::choose(rng);
        self.fx5 = fx5;
        match fx5 {
            Fx5::No => {}
            Fx5::Fungi => self.fungi_amount = rng.gen_range(10.0..=30.0),
            Fx5::GrowFlower => self.grow_flower_amount = rng.gen_range(10.0..=30.0),
        }

        let fx6 = Fx6::choose(rng);
        self.fx6 = fx6;
        match fx6 {
            Fx6::No => {}
            Fx6::Gold => self.gold_silver_amount = rng.gen_range(5.0..=15.0),
            Fx6::Silver => self.gold_silver_amount = rng.gen_range(5.0..=15.0),
            Fx6::RoseGold => self.gold_silver_amount = rng.gen_range(5.0..=15.0),
            Fx6::Bronze => self.gold_silver_amount = rng.gen_range(5.0..=15.0),
            Fx6::Copper => self.gold_silver_amount = rng.gen_range(5.0..=15.0),
        }

        self
    }

    pub fn generate_underwater<R: rand::Rng + ?Sized>(mut self, rng: &mut R) -> Self {
        match self.fx2 {
            Fx2::Underwater => {
                let jellyfish = FxJellyfish::choose(rng);
                self.fx_jellifish = jellyfish;

                self.underwater_fog_amount = rng.gen::<f64>() * 30.0;
                self.background_underwater_color_hue = 38.8;

                let env_light = if self.fx0 == Fx0::Hologram {
                    EnvLight::UnderwaterHologram
                } else {
                    EnvLight::Underwater
                };
                self.env_light = env_light;
            }
            _ => {}
        }
        self
    }

    pub fn generate_helmet_lights<R: rand::Rng + ?Sized>(mut self, rng: &mut R) -> Self {
        match self.helmet_light {
            HelmetLight::Dots | HelmetLight::GlowingEyes => {
                self.eye_color_random_hue = random_hue(rng);
                self.eyes_light_intensity_amount = 100.0;
            }
            _ => {}
        }
        self
    }

    pub fn generate_wedge<R: rand::Rng + ?Sized>(mut self, rng: &mut R) -> Self {
        self.wedgeindex = rng.gen_range(25i64..=1000000000i64);
        self
    }

    pub fn generate_background_color<R: rand::Rng + ?Sized>(mut self, rng: &mut R) -> Self {
        self.background_color_random_hue = random_hue(rng);
        self
    }

    pub fn generate_dress_hue<R: rand::Rng + ?Sized>(mut self, rng: &mut R) -> Self {
        self.dress_color_hue = random_hue(rng);
        self
    }

    pub fn generate_body_material_variation<R: rand::Rng + ?Sized>(mut self, rng: &mut R) -> Self {
        match self.fx0 {
            Fx0::No => {
                self.body_material_variation = BodyMaterialVariations::choose(rng);
            }
            _ => {}
        }
        self
    }

    pub fn generate_marble_variation<R: rand::Rng + ?Sized>(mut self, rng: &mut R) -> Self {
        match self.fx0 {
            Fx0::Marble => {
                self.marble_variation = MarbleVariation::choose(rng);
            }
            _ => {}
        }
        self
    }

    pub fn generate_wood_variation<R: rand::Rng + ?Sized>(mut self, rng: &mut R) -> Self {
        match self.fx0 {
            Fx0::Wood => {
                self.wood_variation = WoodVariation::choose(rng);
            }
            _ => {}
        }
        self
    }

    pub fn glowing_logo<R: rand::Rng + ?Sized>(mut self, rng: &mut R) -> Self {
        self.glowing_logo = GlowingLogo::choose(rng);
        if self.glowing_logo == GlowingLogo::Yes {
            self.logo_hue = random_hue(rng);
        }
        self
    }

    pub fn generate_random_value<R: rand::Rng + ?Sized>(mut self, rng: &mut R) -> Self {
        self.random_value = rng.gen::<f64>() * 360.0;
        self
    }

    pub fn generate_smoke<R: rand::Rng + ?Sized>(mut self, rng: &mut R) -> Self {
        match self.env_light {
            EnvLight::Day | EnvLight::Underwater | EnvLight::UnderwaterHologram => {
                self.smoke_amount = 0.0;
                self.fx3 = Fx3::No;
            }
            EnvLight::Night => {
                self.fx3 = Fx3::smoke_or_not(rng);
                match self.fx3 {
                    Fx3::Smoke => self.smoke_amount = rng.gen_range(25.0..=50.0),
                    Fx3::No => self.smoke_amount = 0.0,
                }
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        vec,
    };

    use serde_json::json;
    use strum::IntoEnumIterator;

    use crate::nft_metadata::EnvReflection;

    use super::*;

    #[test]
    fn test() {
        // generate a base
        let base = RenderParams::generate_base(&mut rand::thread_rng());
        dbg!(&base);

        // store user poses
        let mut poses: HashSet<Pose> = HashSet::new();
        poses.insert(base.pose);

        // get a new random pose not in user poses
        fn get_new_pose(poses: HashSet<Pose>) -> Pose {
            let mut new_poses: Vec<Pose> = Pose::iter().collect();
            new_poses.retain(|p| !poses.contains(p));
            new_poses[rand::thread_rng().gen_range(0..new_poses.len())]
        }
        dbg!(get_new_pose(poses));

        // adjust pose
        #[allow(dead_code)]
        fn adjust_pose(poses: HashSet<Pose>, selected_pose: Pose) -> Option<Pose> {
            // check poses is not empty
            if poses.is_empty() | !poses.contains(&selected_pose) {
                None
            } else {
                Some(selected_pose)
            }
        }

        // tune owned fxs
        // adjust amount
        // toggle on/off

        let mut user_effects: HashMap<String, serde_json::Value> = HashMap::new();
        user_effects.insert("fx0".to_string(), json!([base.fx0.to_string()]));
        user_effects.insert("fx1".to_string(), json!([base.fx1.to_string()]));
        user_effects.insert("fx2".to_string(), json!([base.fx2.to_string()]));
        user_effects.insert("fx4".to_string(), json!([base.fx4.to_string()]));
        user_effects.insert("fx5".to_string(), json!([base.fx5.to_string()]));
        user_effects.insert("fx6".to_string(), json!([base.fx6.to_string()]));
        user_effects.insert(
            "fx_jellifish".to_string(),
            json!([base.fx_jellifish.to_string()]),
        );
        user_effects.insert(
            "fx_lineart_helper".to_string(),
            json!([base.fx_lineart_helper.to_string()]),
        );
        user_effects.insert("fx1a".to_string(), json!([base.fx1a.to_string()]));
        dbg!(&user_effects);

        // convert user_effects to json
        let user_effects_json = serde_json::to_value(&user_effects).unwrap();
        dbg!(&user_effects_json);

        // effect_lottery
        // read user effects and get a new random effect not in user effects
        fn get_new_effect(user_effects: &HashMap<String, serde_json::Value>) -> String {
            fn get_effect_names(
                user_effects: &HashMap<String, serde_json::Value>,
                key: &str,
            ) -> Vec<String> {
                user_effects
                    .get(key)
                    .map(|effect| {
                        effect
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|value| value.as_str().unwrap().to_string())
                            .collect::<Vec<String>>()
                    })
                    .unwrap_or_default()
            }

            // get fx0 not in user_effects
            let mut fx0: Vec<String> = Fx0::iter()
                .filter(|fx| *fx != Fx0::No)
                .map(|fx| fx.to_string())
                .collect::<Vec<String>>();

            let effects = get_effect_names(user_effects, "fx0");
            fx0.retain(|fx| !effects.contains(fx));
            fx0[rand::thread_rng().gen_range(0..fx0.len())].to_string();

            // get fx1 not in user_effects
            let mut fx1: Vec<String> = Fx1::iter()
                .filter(|fx| *fx != Fx1::No)
                .map(|fx| fx.to_string())
                .collect::<Vec<String>>();
            let effects = get_effect_names(user_effects, "fx1");
            fx1.retain(|fx| !effects.contains(fx));
            fx1[rand::thread_rng().gen_range(0..fx1.len())].to_string();

            //join the two new_effects
            let new_effects = [fx0, fx1].concat();

            new_effects[rand::thread_rng().gen_range(0..new_effects.len())].to_string()
        }

        let _new_effect = dbg!(get_new_effect(&user_effects));
        //

        // add new fx
        #[allow(dead_code)]
        fn add_new_effect_to_base(mut base: RenderParams, new_effect: &str) -> RenderParams {
            //find the new_effect in the RenderParams enum
            match new_effect {
                "Hologram" => base.fx0 = Fx0::Hologram,
                "Xray" => base.fx0 = Fx0::Xray,
                "SoapBubble" => base.fx0 = Fx0::SoapBubble,
                "Pixel" => base.fx0 = Fx0::Pixel,
                "Melted" => base.fx1 = Fx1::Melted,
                "Disintegration" => base.fx1 = Fx1::Disintegration,
                "Butterflies" => base.fx2 = Fx2::Butterflies,
                "Underwater" => base.fx2 = Fx2::Underwater,
                "Fireflyies" => base.fx2 = Fx2::Fireflyies,
                "Fall" => base.fx2 = Fx2::Fall,
                "Ladybag" => base.fx2 = Fx2::Ladybag,
                "Spring" => base.fx2 = Fx2::Spring,
                "Frozen" => base.fx4 = Fx4::Frozen,
                "Rain" => base.fx4 = Fx4::Rain,
                "Fungi" => base.fx5 = Fx5::Fungi,
                "GrowFlower" => base.fx5 = Fx5::GrowFlower,
                "Gold" => base.fx6 = Fx6::Gold,
                "Silver" => base.fx6 = Fx6::Silver,
                "LineartMinimalistic" => base.fx1a = Fx1a::LineartMinimalistic,
                "LineartHeavy" => base.fx1a = Fx1a::LineartHeavy,
                _ => {}
            }

            //add the new_effect to the base
            //return the base
            base
        }

        // dbg!(add_new_effect_to_base(base, &new_effect));
        // togg
        // dbg!(base);
    }

    #[test]
    fn iterate() {
        let count = BodyType::iter().count()
            * Pose::iter().count()
            * HelmetType::iter().count()
            * HelmetLight::iter().count()
            * (Fx0::iter().count() - 2)
            * Fx1::iter().count()
            * Fx1a::iter().count()
            * Fx2::iter().count()
            * Fx3::iter().count()
            * Fx4::iter().count()
            * Fx5::iter().count()
            * Fx6::iter().count()
            * FxJellyfish::iter().count()
            * FxLineartHelper::iter().count()
            * EnvLight::iter().count()
            * EnvReflection::iter().count()
            * LightReflectionMult::iter().count();
        println!("{}", count);
    }
}
