//! Crop menu scenery to cover the viewport without stretching its proportions.
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub(crate) fn update_menu_backdrop_crop(
    images: Res<Assets<Image>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut backdrops: Query<(&Name, &mut ImageNode)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let viewport = Vec2::new(window.width(), window.height());
    if viewport.x <= 0.0 || viewport.y <= 0.0 {
        return;
    }
    for (name, mut node) in &mut backdrops {
        if !matches!(
            name.as_str(),
            "Godot Main Menu Background" | "Menu Background"
        ) {
            continue;
        }
        let Some(image) = images.get(&node.image) else {
            continue;
        };
        let size = Vec2::new(image.width() as f32, image.height() as f32);
        if size.x <= 0.0 || size.y <= 0.0 {
            continue;
        }
        let aspect = viewport.x / viewport.y;
        let crop = if size.x / size.y > aspect {
            Vec2::new(size.y * aspect, size.y)
        } else {
            Vec2::new(size.x, size.x / aspect)
        };
        let rect = Rect::from_center_size(size * 0.5, crop);
        if node.rect != Some(rect) {
            node.rect = Some(rect);
        }
    }
}
