//! Browser resource preparation. Menus load core art; matches wait for the shared pack.
use crate::*;
use bevy::asset::io::{
    AssetReader, AssetReaderError, AssetSourceBuilder, AssetSourceId, PathStream, Reader,
    wasm::HttpWasmAssetReader,
};
use bevy::state::state::StateTransitionSystems;
use js_sys::wasm_bindgen::JsCast;
use std::{
    future::poll_fn,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Poll, Waker},
};

#[derive(Default)]
struct Gate {
    open: AtomicBool,
    waiters: Mutex<Vec<Waker>>,
}
impl Gate {
    async fn wait(&self) {
        poll_fn(|cx| {
            let mut list = self.waiters.lock().unwrap();
            if self.open.load(Ordering::Acquire) {
                Poll::Ready(())
            } else {
                if !list.iter().any(|w| w.will_wake(cx.waker())) {
                    list.push(cx.waker().clone());
                }
                Poll::Pending
            }
        })
        .await
    }
    fn release(&self) {
        self.open.store(true, Ordering::Release);
        for w in self.waiters.lock().unwrap().drain(..) {
            w.wake();
        }
    }
}
struct ReaderGate {
    inner: HttpWasmAssetReader,
    gate: Arc<Gate>,
}
fn core(path: &Path) -> bool {
    [
        "fonts",
        "ui",
        "campaign",
        "data",
        "migration",
        "translations",
        "logos",
    ]
    .iter()
    .any(|p| path.starts_with(p))
}
impl AssetReader for ReaderGate {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        if !core(path) {
            self.gate.wait().await;
        }
        self.inner.read(path).await
    }
    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        if !core(path) {
            self.gate.wait().await;
        }
        self.inner.read_meta(path).await
    }
    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        self.inner.read_directory(path).await
    }
    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        self.inner.is_directory(path).await
    }
}
#[derive(Resource, Default)]
struct Preparation {
    gate: Arc<Gate>,
    target: Option<AppScreen>,
    handles: Vec<UntypedHandle>,
    decoding: bool,
    ready: bool,
    entered: bool,
}
fn call(name: &str, value: &js_sys::wasm_bindgen::JsValue) {
    if let Some(window) = web_sys::window()
        && let Ok(callback) = js_sys::Reflect::get(window.as_ref(), &name.into())
        && let Some(callback) = callback.dyn_ref::<js_sys::Function>()
    {
        let _ = callback.call1(window.as_ref(), value);
    }
}
pub(crate) fn open_downloads() {
    call("rtsOpenDownloads", &0.into());
}
pub(crate) fn install(app: &mut App) {
    let preparation = Preparation::default();
    let gate = preparation.gate.clone();
    app.register_asset_source(
        AssetSourceId::Default,
        AssetSourceBuilder::new(move || {
            Box::new(ReaderGate {
                inner: HttpWasmAssetReader::new("assets"),
                gate: gate.clone(),
            })
        }),
    )
    .insert_resource(preparation)
    .add_systems(
        StateTransition,
        intercept_match.before(StateTransitionSystems::DependentTransitions),
    )
    .add_systems(Update, finish_preparation)
    .add_systems(PreUpdate, (sync_viewport, touch_camera).chain())
    .add_systems(
        Startup,
        restore_web_language.after(restore_player_preferences),
    )
    .add_systems(Update, publish_web_language.after(sync_locale));
}
fn restore_web_language(mut locale: ResMut<Locale>, mut options: ResMut<MenuOptionsState>) {
    if let Some(language) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
        .and_then(|e| e.get_attribute("lang"))
    {
        locale.0 = if language == "zh-CN" {
            Language::Zh
        } else {
            Language::En
        };
        options.language = locale.0;
        set_current_language(locale.0);
    }
}
fn publish_web_language(locale: Res<Locale>) {
    if locale.is_changed() {
        call(
            "rtsSetLanguage",
            &if locale.0 == Language::Zh {
                "zh-CN"
            } else {
                "en"
            }
            .into(),
        );
    }
}
fn intercept_match(mut next: ResMut<NextState<AppScreen>>, mut preparation: ResMut<Preparation>) {
    let target = match &*next {
        NextState::Pending(screen @ (AppScreen::InMatch | AppScreen::OnlineLobby)) => *screen,
        _ => return,
    };
    if preparation.ready {
        return;
    }
    *next = NextState::Unchanged;
    if preparation.target.is_none() {
        preparation.target = Some(target);
        call("rtsPrepareMatch", &0.into());
    }
}
fn finish_preparation(
    mut preparation: ResMut<Preparation>,
    assets: Res<AssetServer>,
    mut next: ResMut<NextState<AppScreen>>,
    screen: Res<State<AppScreen>>,
) {
    web_canvas_attribute("data-app-screen", &format!("{:?}", screen.get()));
    if matches!(screen.get(), AppScreen::InMatch | AppScreen::OnlineLobby) {
        preparation.entered = true;
    }
    if preparation.entered && *screen.get() == AppScreen::MainMenu {
        preparation.entered = false;
        preparation.ready = false;
        call("rtsReleaseMatch", &0.into());
    }
    let Some(target) = preparation.target else {
        return;
    };
    let state = web_sys::window()
        .and_then(|w| js_sys::Reflect::get(w.as_ref(), &"rtsMatchDownload".into()).ok())
        .and_then(|v| js_sys::Reflect::get(&v, &"status".into()).ok())
        .and_then(|v| v.as_string());
    if state.as_deref() == Some("cancelled") {
        preparation.target = None;
        return;
    }
    if state.as_deref() != Some("ready") {
        return;
    }
    if !preparation.decoding {
        preparation.gate.release();
        preparation.decoding = true;
        preparation
            .handles
            .push(load_armor_surface(&assets).untyped());
        preparation.handles.push(
            assets
                .load::<WorldAsset>(GltfAssetLabel::Scene(0).from_asset(DEFAULT_MODEL_FALLBACK))
                .untyped(),
        );
        for entity in registry::ENTITY_DEFS {
            for part in entity.render_parts {
                preparation.handles.push(
                    assets
                        .load::<WorldAsset>(GltfAssetLabel::Scene(0).from_asset(part.model))
                        .untyped(),
                );
            }
        }
    }
    if preparation.handles.iter().any(|h| {
        matches!(
            assets.get_load_state(h.id()),
            Some(bevy::asset::LoadState::Failed(_))
        )
    }) {
        call("rtsAssetsFailed", &0.into());
        preparation.decoding = false;
        for handle in preparation.handles.drain(..) {
            if let Some(path) = assets.get_path(handle.id()) {
                assets.reload(path);
            }
        }
        return;
    }
    if preparation
        .handles
        .iter()
        .all(|h| assets.is_loaded_with_dependencies(h.id()))
    {
        call("rtsAssetsDecoded", &0.into());
        preparation.ready = true;
        preparation.target = None;
        next.set(target);
    }
}

fn touch_camera(
    mut camera: ResMut<RtsCamera>,
    zones: Res<HudHitZones>,
    menu: Res<MatchMenuState>,
    views: Query<&GlobalTransform, With<MainCamera>>,
) {
    let Some(window) = web_sys::window() else {
        return;
    };
    if touch_device() {
        let rects = js_sys::Array::new();
        for (min, max) in &zones.world_rects {
            let r = js_sys::Array::new();
            for n in [min.x, min.y, max.x, max.y] {
                r.push(&n.into());
            }
            rects.push(&r);
        }
        let _ = js_sys::Reflect::set(window.as_ref(), &"rtsTouchHudRects".into(), &rects);
        let _ = js_sys::Reflect::set(
            window.as_ref(),
            &"rtsTouchWorldEnabled".into(),
            &(!menu.visible).into(),
        );
    }
    let Ok(v) = js_sys::Reflect::get(window.as_ref(), &"rtsTouchCamera".into()) else {
        return;
    };
    if !v.is_object() {
        return;
    }
    let read = |name: &str| {
        js_sys::Reflect::get(&v, &name.into())
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32
    };
    let (x, y, z) = (read("x"), read("y"), read("zoom"));
    if x.abs() + y.abs() + z.abs() > 0.0 {
        let scale = camera.distance * 0.003;
        let (right, up) = views
            .single()
            .map(|view| {
                let r = view.right().as_vec3();
                let u = view.up().as_vec3();
                (
                    Vec3::new(r.x, 0.0, r.z).normalize_or_zero(),
                    Vec3::new(u.x, 0.0, u.z).normalize_or_zero(),
                )
            })
            .unwrap_or((Vec3::X, -Vec3::Z));
        camera.focus += (-right * x + up * y) * scale;
        camera.distance = safe_camera_distance(camera.distance + z);
        camera.pending_jump = true;
        for name in ["x", "y", "zoom"] {
            let _ = js_sys::Reflect::set(&v, &name.into(), &0.into());
        }
    }
}

pub(crate) fn touch_device() -> bool {
    web_sys::window()
        .and_then(|w| js_sys::Reflect::get(w.as_ref(), &"rtsTouchDevice".into()).ok())
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

// ResizeObserver's device-pixel box can differ from CSS coordinates under
// browser zoom. Keep the renderer and pointer picking in the same coordinate space.
fn sync_viewport(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    let Some(browser) = web_sys::window() else { return; };
    let Some(canvas) = browser.document().and_then(|d| d.get_element_by_id("bevy-canvas")) else { return; };
    let rect = canvas.get_bounding_client_rect();
    let width = rect.width();
    let height = rect.height();
    let scale = ((browser.device_pixel_ratio() * 1000.0).round() / 1000.0) as f32;
    if width <= 0.0 || height <= 0.0 || !scale.is_finite() || scale <= 0.0 {
        return;
    }
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    let physical_width = (width * scale as f64).round() as u32;
    let physical_height = (height * scale as f64).round() as u32;
    if window.resolution.scale_factor_override() != Some(scale) {
        window.resolution.set_scale_factor_override(Some(scale));
    }
    if window.physical_width() != physical_width || window.physical_height() != physical_height {
        window
            .resolution
            .set_physical_resolution(physical_width, physical_height);
    }
}
