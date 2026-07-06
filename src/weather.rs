//! Weather: a deterministic ambient cycle (clear ↔ overcast / rain /
//! sandstorm) that cross-fades sun intensity, ambient light and distance fog,
//! draws slanted rain streaks in the camera's view, and blows sand puffs
//! across the field during storms. Purely visual — no gameplay effect — and
//! fully deterministic (hash-scheduled), so replays and captures agree.

use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;

use crate::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum WeatherKind {
    Clear,
    Overcast,
    Rain,
    Sandstorm,
}

/// Environment parameters for one weather kind.
#[derive(Clone, Copy)]
pub(crate) struct WeatherParams {
    pub(crate) sun_lux: f32,
    pub(crate) ambient: f32,
    pub(crate) fog_density: f32,
    pub(crate) fog_color: Color,
    pub(crate) rain: f32,
    pub(crate) dust: f32,
}

impl WeatherKind {
    pub(crate) fn params(self) -> WeatherParams {
        match self {
            Self::Clear => WeatherParams {
                sun_lux: 11_000.0,
                ambient: 150.0,
                fog_density: 0.0,
                fog_color: Color::srgb(0.8, 0.7, 0.6),
                rain: 0.0,
                dust: 0.0,
            },
            Self::Overcast => WeatherParams {
                sun_lux: 7_200.0,
                ambient: 120.0,
                fog_density: 0.004,
                fog_color: Color::srgb(0.66, 0.66, 0.68),
                rain: 0.0,
                dust: 0.0,
            },
            Self::Rain => WeatherParams {
                sun_lux: 5_400.0,
                ambient: 100.0,
                fog_density: 0.010,
                fog_color: Color::srgb(0.55, 0.6, 0.66),
                rain: 1.0,
                dust: 0.0,
            },
            Self::Sandstorm => WeatherParams {
                sun_lux: 7_000.0,
                ambient: 110.0,
                fog_density: 0.022,
                fog_color: Color::srgb(0.8, 0.64, 0.47),
                rain: 0.0,
                dust: 1.0,
            },
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let (a, b) = (a.to_linear(), b.to_linear());
    Color::linear_rgb(
        lerp(a.red, b.red, t),
        lerp(a.green, b.green, t),
        lerp(a.blue, b.blue, t),
    )
}

/// How long each weather phase lasts before the next roll, and how long the
/// cross-fade between phases takes.
const WEATHER_PHASE_MIN_SECONDS: f32 = 100.0;
const WEATHER_PHASE_VARIANCE_SECONDS: f32 = 70.0;
const WEATHER_FADE_SECONDS: f32 = 9.0;

#[derive(Resource)]
pub(crate) struct WeatherState {
    pub(crate) previous: WeatherKind,
    pub(crate) current: WeatherKind,
    /// 0..1 cross-fade from `previous` to `current`.
    pub(crate) blend: f32,
    pub(crate) phase_remaining: f32,
    pub(crate) step: u32,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            previous: WeatherKind::Clear,
            current: WeatherKind::Clear,
            blend: 1.0,
            phase_remaining: WEATHER_PHASE_MIN_SECONDS,
            step: 0,
        }
    }
}

impl WeatherState {
    /// The blended environment parameters for this frame.
    pub(crate) fn blended(&self) -> WeatherParams {
        let (a, b, t) = (
            self.previous.params(),
            self.current.params(),
            self.blend.clamp(0.0, 1.0),
        );
        WeatherParams {
            sun_lux: lerp(a.sun_lux, b.sun_lux, t),
            ambient: lerp(a.ambient, b.ambient, t),
            fog_density: lerp(a.fog_density, b.fog_density, t),
            fog_color: lerp_color(a.fog_color, b.fog_color, t),
            rain: lerp(a.rain, b.rain, t),
            dust: lerp(a.dust, b.dust, t),
        }
    }
}

fn weather_hash01(step: u32, salt: u32) -> f32 {
    let mut h = (step as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(salt as u64)
        .wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 31;
    ((h & 0xffff) as f32) / 65_535.0
}

/// Advances the phase clock and rolls the next weather. The cycle alternates
/// clear spells with one weather event (overcast / rain / sandstorm), so the
/// battlefield never sits in permanent soup.
pub(crate) fn tick_weather(time: Res<Time>, mut weather: ResMut<WeatherState>) {
    let dt = time.delta_secs();
    if weather.blend < 1.0 {
        weather.blend = (weather.blend + dt / WEATHER_FADE_SECONDS).min(1.0);
    }
    weather.phase_remaining -= dt;
    if weather.phase_remaining > 0.0 {
        return;
    }
    weather.step = weather.step.wrapping_add(1);
    let step = weather.step;
    weather.previous = weather.current;
    weather.current = if weather.current != WeatherKind::Clear {
        WeatherKind::Clear
    } else {
        match (weather_hash01(step, 7) * 3.0) as u32 {
            0 => WeatherKind::Overcast,
            1 => WeatherKind::Rain,
            _ => WeatherKind::Sandstorm,
        }
    };
    weather.blend = 0.0;
    weather.phase_remaining =
        WEATHER_PHASE_MIN_SECONDS + weather_hash01(step, 13) * WEATHER_PHASE_VARIANCE_SECONDS;
}

/// Pushes the blended weather into the sun, ambient light and camera fog.
pub(crate) fn apply_weather_environment(
    mut commands: Commands,
    weather: Res<WeatherState>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut suns: Query<&mut DirectionalLight>,
    mut cameras: Query<(Entity, Option<&mut DistanceFog>), With<MainCamera>>,
) {
    let params = weather.blended();
    for mut sun in &mut suns {
        sun.illuminance = params.sun_lux;
    }
    ambient.brightness = params.ambient;
    for (camera, fog) in &mut cameras {
        match fog {
            Some(mut fog) => {
                fog.color = params.fog_color;
                fog.falloff = FogFalloff::Exponential {
                    density: params.fog_density,
                };
            }
            None => {
                commands.entity(camera).try_insert(DistanceFog {
                    color: params.fog_color,
                    directional_light_color: Color::NONE,
                    directional_light_exponent: 8.0,
                    falloff: FogFalloff::Exponential {
                        density: params.fog_density,
                    },
                    ..default()
                });
            }
        }
    }
}

/// Slanted rain streaks inside the camera's view, deterministic per index so
/// the sheet of rain is stable frame-to-frame.
pub(crate) fn draw_rain(
    weather: Res<WeatherState>,
    time: Res<Time>,
    cameras: Query<&GlobalTransform, With<MainCamera>>,
    mut gizmos: Gizmos,
) {
    let strength = weather.blended().rain;
    if strength <= 0.01 {
        return;
    }
    let Ok(camera) = cameras.single() else {
        return;
    };
    // Center the sheet on the ground point the camera looks at.
    let forward = camera.forward();
    let center =
        camera.translation() + forward * (camera.translation().y / forward.y.abs().max(0.1));
    let t = time.elapsed_secs();
    let streaks = (220.0 * strength) as u32;
    let slant = Vec3::new(0.16, -1.0, 0.1).normalize();
    for index in 0..streaks {
        let hx = weather_hash01(index, 101);
        let hz = weather_hash01(index, 202);
        let hp = weather_hash01(index, 303);
        // Cover the whole visible ground: the isometric view shows more world
        // toward the camera (+z of the look point), and tall streaks read
        // up-screen, so bias the sheet down-screen and keep tops low.
        let x = center.x + (hx - 0.5) * 34.0;
        let z = center.z + (hz - 0.28) * 26.0;
        // Each streak falls from ~6m and wraps around.
        let fall = ((t * 8.0 + hp * 6.0) % 6.0) / 6.0;
        let top = Vec3::new(x, 6.0 * (1.0 - fall), z);
        gizmos.line(
            top,
            top + slant * 0.8,
            Color::srgba(0.7, 0.78, 0.9, 0.30 * strength),
        );
    }
}

/// A wind-blown sand puff during sandstorms.
#[derive(Component)]
pub(crate) struct DustPuff {
    pub(crate) age: f32,
    pub(crate) ttl: f32,
    pub(crate) velocity: Vec3,
    pub(crate) material: Handle<StandardMaterial>,
}

pub(crate) fn emit_dust_puffs(
    mut commands: Commands,
    weather: Res<WeatherState>,
    time: Res<Time>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut puff_mesh: Local<Option<Handle<Mesh>>>,
    mut next_emit: Local<f32>,
    mut emit_step: Local<u32>,
    cameras: Query<&GlobalTransform, With<MainCamera>>,
) {
    let dust = weather.blended().dust;
    if dust <= 0.05 {
        return;
    }
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    let Ok(camera) = cameras.single() else {
        return;
    };
    *next_emit -= time.delta_secs();
    if *next_emit > 0.0 {
        return;
    }
    *next_emit = 0.10 / dust.max(0.1);
    *emit_step = emit_step.wrapping_add(1);
    let step = *emit_step;
    let forward = camera.forward();
    let center =
        camera.translation() + forward * (camera.translation().y / forward.y.abs().max(0.1));
    let spawn = center
        + Vec3::new(
            -20.0 + weather_hash01(step, 11) * 6.0,
            0.4 + weather_hash01(step, 23) * 1.8,
            (weather_hash01(step, 37) - 0.5) * 26.0,
        );
    let mesh = puff_mesh
        .get_or_insert_with(|| meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap()))
        .clone();
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.8, 0.66, 0.48, 0.16 * dust),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    commands.spawn((
        Name::new("Dust puff"),
        bevy::light::NotShadowCaster,
        Mesh3d(mesh),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(spawn).with_scale(Vec3::splat(0.5)),
        DustPuff {
            age: 0.0,
            ttl: 2.4,
            velocity: Vec3::new(9.0 + weather_hash01(step, 41) * 4.0, 0.1, 1.2),
            material,
        },
        MatchScopedEntity,
    ));
}

pub(crate) fn update_dust_puffs(
    mut commands: Commands,
    time: Res<Time>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut puffs: Query<(Entity, &mut DustPuff, &mut Transform)>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let dt = time.delta_secs();
    for (entity, mut puff, mut transform) in &mut puffs {
        puff.age += dt;
        let life = (puff.age / puff.ttl).clamp(0.0, 1.0);
        if life >= 1.0 {
            materials.remove(&puff.material);
            commands.entity(entity).try_despawn();
            continue;
        }
        let velocity = puff.velocity;
        transform.translation += velocity * dt;
        transform.scale = Vec3::splat(0.5 + life * 1.1);
        if let Some(mut material) = materials.get_mut(&puff.material) {
            let fade = if life < 0.2 {
                life / 0.2
            } else {
                1.0 - (life - 0.2) / 0.8
            };
            material.base_color.set_alpha(0.16 * fade);
        }
    }
}
