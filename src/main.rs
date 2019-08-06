extern crate amethyst;

#[macro_use]
extern crate log;
#[macro_use]
extern crate specs_derive;

use amethyst::{
    animation::AnimationBundle,
    assets::{PrefabLoaderSystem, Processor},
    ecs::System,
    core::transform::TransformBundle,
    input::{InputBundle, StringBindings},
    renderer::{
        plugins::{RenderFlat2D, RenderToWindow},
        sprite::SpriteRender,
        types::DefaultBackend,
        RenderingBundle,
    },
    utils::application_root_dir,
    Application, GameDataBuilder,
};

mod components;
mod entities;
mod resources;
mod states;
mod systems;

use components::{AnimationId, AnimationPrefabData};
use resources::Map;
use systems::*;
#[cfg(feature="time_metrics")]
use systems::time_metrics::{TimeMetricsWriterSystem, TimeMetricsWrapperSystem};

trait GameDataBuilderExt<'a>{
    #[cfg(not(feature="time_metrics"))]
    fn with_wrapped<T>(self, system: T, name: &'static str, dependencies: &[&'static str]) -> Self
        where for<'c> T: System<'c> + 'a + Send;

    #[cfg(feature="time_metrics")]
    fn with_wrapped<T>(self, system: T, name: &'static str, dependencies: &[&'static str]) -> Self
        where for<'c> TimeMetricsWrapperSystem<T>: System<'c> + 'a + Send;
}

impl<'a> GameDataBuilderExt<'a> for GameDataBuilder<'a, '_>{
    #[cfg(not(feature="time_metrics"))]
    fn with_wrapped<T>(self, system: T, name: &'static str, dependencies: &[&'static str]) -> Self
        where for<'c> T: System<'c> + 'a + Send
    {
        self.with(system, name, dependencies)
    }

    #[cfg(feature="time_metrics")]
    fn with_wrapped<T>(self, system: T, name: &'static str, dependencies: &[&'static str]) -> Self
        where for<'c> TimeMetricsWrapperSystem<T>: System<'c> + 'a + Send
    {
        self.with(TimeMetricsWrapperSystem::new(system, name), name, dependencies)
    }
}

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let root = application_root_dir()?;
    let display_config_path = root.join("resources/display_config.ron");
    let assets_path = root.join("assets");
    let input_bundle = InputBundle::<StringBindings>::new()
        .with_bindings_from_file(root.join("resources/bindings_config.ron"))?;

    let game_data = GameDataBuilder::default()
        .with(
            PrefabLoaderSystem::<AnimationPrefabData>::default(),
            "scene_loader",
            &[],
        )
        .with_bundle(AnimationBundle::<AnimationId, SpriteRender>::new(
            "sprite_animation_control",
            "sprite_sampler_interpolation",
        ))?
        .with_bundle(
            TransformBundle::new()
                .with_dep(&["sprite_animation_control", "sprite_sampler_interpolation"]),
        )?
        .with_bundle(input_bundle)?
        .with(Processor::<Map>::new(), "map_processor", &[])
        .with(MarineAccelerationSystem, "marine_acceleration_system", &[])
        .with(
            AttackSystem,
            "attack_system",
            &["marine_acceleration_system"],
        )
        .with(
            CollisionSystem,
            "collision_system",
            &["marine_acceleration_system"],
        )
        .with(
            BulletCollisionSystem,
            "bullet_collision_system",
            &["collision_system"],
        )
        .with(
            BulletImpactAnimationSystem,
            "bullet_impact_animation_system",
            &["bullet_collision_system"],
        )
        .with(
            PincerCollisionSystem,
            "pincer_collision_system",
            &["collision_system"],
        )
        .with(
            PincerAnimationSystem,
            "pincer_animation_system",
            &["pincer_collision_system"],
        )
        .with(ExplosionAnimationSystem, "explosion_animation_system", &[])
        .with(ParallaxSystem, "parallax_system", &[])
        .with(
            MotionSystem,
            "motion_system",
            &["collision_system", "parallax_system"],
        )
        .with_wrapped(
            MarineAnimationSystem,
            "marine_animation_system",
            &["collision_system"],
        )
        .with_wrapped(AnimationControlSystem, "animation_control_system", &[])
        .with_wrapped(DirectionSystem, "direction_system", &[])
        .with_wrapped(
            CameraMotionSystem,
            "camera_motion_system",
            &["collision_system"],
        )
        .with_bundle(
            RenderingBundle::<DefaultBackend>::new()
                // The RenderToWindow plugin provides all the scaffolding for opening a window and drawing on it
                .with_plugin(
                    RenderToWindow::from_config_path(display_config_path)
                        .with_clear([0.008, 0.043, 0.067, 1.0]),
                )
                .with_plugin(RenderFlat2D::default()),
        )?;
    #[cfg(feature="time_metrics")]
    let game_data = game_data
        .with_wrapped(TimeMetricsWriterSystem::with_file("time_metrics.log").expect("Cannot open time_metrics.log"), "time_metrics", &[]);
    let mut game =
        Application::build(assets_path, states::LoadState::default())?.build(game_data)?;

    game.run();

    Ok(())
}
