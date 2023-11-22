use super::{
    BodyType, EnvLight, Fx0, Fx1, Fx1a, Fx2, Fx4, Fx5, Fx6, FxJellyfish, FxLineartHelper,
    HelmetLight, HelmetType, LightReflectionMult, Pose, RenderParams,
};
use rand::Rng;

impl RenderParams {
    pub fn generate_base() -> Self {
        let body_type = BodyType::seed();
        let pose = Pose::seed();
        let helmet_type = HelmetType::seed();
        let helmet_light = HelmetLight::seed();
        let fx0 = Fx0::seed();

        Self {
            body_type,
            pose,
            helmet_type,
            helmet_light,
            fx0,
            ..<_>::default()
        }
        .adjust_base()
        .generate_line_art()
        .generate_fx()
        .generate_underwater()
        .generate_background_color()
        .generate_dress_hue()
        .generate_helmet_lights()
        .generate_wedge()
    }

    pub fn generate_line_art(mut self) -> Self {
        match self.fx0 {
            Fx0::LineartBase => {
                let line_art = Fx1a::seed();
                self.lineart_amount = rand::random::<f64>() * 100.0;

                self.fx1a = line_art;
                match line_art {
                    Fx1a::LineartMinimalistic => {
                        let fx_lineart_helper = FxLineartHelper::Zero;
                        self.fx_lineart_helper = fx_lineart_helper;
                    }
                    Fx1a::LineartHeavy => {
                        let fx_lineart_helper = FxLineartHelper::Zero;
                        self.fx_lineart_helper = fx_lineart_helper;
                    }
                    Fx1a::No => {
                        let fx_lineart_helper = FxLineartHelper::Zero;
                        self.fx_lineart_helper = fx_lineart_helper;
                    }
                }
            }
            _ => {
                let line_art = Fx1a::seed();
                self.fx1a = line_art;
                match line_art {
                    Fx1a::No => {
                        let fx_lineart_helper = FxLineartHelper::seed();
                        self.fx_lineart_helper = fx_lineart_helper;
                    }
                    Fx1a::LineartMinimalistic => {
                        let fx_lineart_helper = FxLineartHelper::One;
                        self.fx_lineart_helper = fx_lineart_helper;
                    }
                    Fx1a::LineartHeavy => {
                        let fx_lineart_helper = FxLineartHelper::Zero;
                        self.fx_lineart_helper = fx_lineart_helper;
                    }
                }
            }
        }
        self
    }

    pub fn adjust_base(mut self) -> Self {
        match self.fx0 {
            Fx0::Hologram => {
                self.env_light = EnvLight::Night;
                self.hologram_amount = rand::thread_rng().gen_range(25.0..=100.0)
            }
            Fx0::Xray => {
                self.xray_skeleton_particles_amount = rand::thread_rng().gen_range(25.0..=100.0);
                self.xray_body_amount = rand::thread_rng().gen_range(25.0..=100.0);
            }
            Fx0::SoapBubble => {
                self.soap_bubble_intensity_amount = rand::thread_rng().gen_range(25.0..=100.0);
                self.soap_bubble_roughness_amount = rand::thread_rng().gen_range(25.0..=100.0);
                self.light_reflection_mult = LightReflectionMult::Two;
            }
            Fx0::Pixel => {
                self.pixel_amount = rand::random::<f64>() * 20.0;
            }
            _ => {}
        }
        self
    }

    pub fn generate_fx(mut self) -> Self {
        let fx1 = Fx1::seed();

        self.fx1 = fx1;
        match fx1 {
            Fx1::No => {}
            Fx1::Melted => {
                self.melt_amount = rand::random::<f64>() * 30.0;
                self.melting_glow_amount = 50.0;
            }
            Fx1::Disintegration => self.disintegration_amount = rand::random::<f64>() * 30.0,
        }

        let fx2 = Fx2::seed();
        self.fx2 = fx2;
        match fx2 {
            Fx2::No => {}
            Fx2::Butterflies => self.butterfly_amount = rand::random::<f64>() * 30.0,
            Fx2::Underwater => {} // underwater is handled separately
            Fx2::Fireflyies => self.firefly_amount = rand::random::<f64>() * 30.0,
            Fx2::Fall => self.fall_amount = rand::random::<f64>() * 30.0,
            Fx2::Ladybag => self.ladybag_amount = rand::random::<f64>() * 30.0,
            Fx2::Spring => self.spring_amount = rand::random::<f64>() * 30.0,
        }

        let fx4 = Fx4::seed();
        self.fx4 = fx4;
        match fx4 {
            Fx4::No => {}
            Fx4::Frozen => self.frozen_amount = rand::random::<f64>() * 30.0,
            Fx4::Rain => self.rain_amount = rand::random::<f64>() * 40.0,
        }

        let fx5 = Fx5::seed();
        self.fx5 = fx5;
        match fx5 {
            Fx5::No => {}
            Fx5::Fungi => self.fungi_amount = rand::random::<f64>() * 30.0,
            Fx5::GrowFlower => self.grow_flower_amount = rand::random::<f64>() * 30.0,
        }

        let fx6 = Fx6::seed();
        self.fx6 = fx6;
        match fx6 {
            Fx6::No => {}
            Fx6::Gold => self.gold_silver_amount = rand::random::<f64>() * 30.0,
            Fx6::Silver => self.gold_silver_amount = rand::random::<f64>() * 30.0,
        }

        self
    }

    pub fn generate_underwater(mut self) -> Self {
        match self.fx2 {
            Fx2::Underwater => {
                let jellyfish = FxJellyfish::seed();
                self.fx_jellifish = jellyfish;

                self.underwater_fog_amount = rand::random::<f64>() * 30.0;
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

    pub fn generate_helmet_lights(mut self) -> Self {
        match self.helmet_light {
            HelmetLight::Dots | HelmetLight::GlowingEyes => {
                self.eye_color_random_hue = rand::random::<f64>() * 360.0;
                self.eyes_light_intensity_amount = 100.0;
            }
            _ => {}
        }
        self
    }

    pub fn generate_wedge(mut self) -> Self {
        self.wedgeindex = rand::thread_rng().gen_range(25i64..=1000000000i64);
        self
    }

    pub fn generate_background_color(mut self) -> Self {
        self.background_color_random_hue = rand::random::<f64>() * 360.0;
        self
    }

    pub fn generate_dress_hue(mut self) -> Self {
        self.dress_color_hue = rand::random::<f64>() * 360.0;
        self
    }

    //smoke, env_reflection, light_reflection, env_light
    //
    //
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        vec,
    };

    use serde_json::json;
    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    fn test() {
        // generate a base
        let base = RenderParams::generate_base();

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
}
