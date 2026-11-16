//! Short landscape front menu retains full touch targets instead of scaling everything.
use crate::*;
#[derive(Component)]
pub(crate) struct OriginalLayout(Node);
pub(crate) fn compact_front_menu(
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut nodes: Query<(
        Entity,
        &mut Node,
        Option<&Name>,
        Option<&FrontMenuButton>,
        Option<&OriginalLayout>,
    )>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let compact = window.height() < 600.0 || window.width() < 800.0;
    for (entity, mut node, name, button, original) in &mut nodes {
        let label = name.map(|n| n.as_str()).unwrap_or("");
        if button.is_none()
            && !matches!(
                label,
                "Front Briefing" | "Front Commands" | "Godot Style Command Menu"
            )
        {
            continue;
        }
        let base = original
            .map(|v| v.0.clone())
            .unwrap_or_else(|| node.clone());
        if original.is_none() {
            commands.entity(entity).insert(OriginalLayout(base.clone()));
        }
        let mut layout = base;
        if compact {
            match label {
                "Front Briefing" => layout.display = Display::None,
                "Godot Style Command Menu" => {
                    layout.padding = UiRect::all(px(12));
                    layout.justify_content = JustifyContent::Center;
                }
                "Front Commands" => {
                    layout.width = px(window.width().min(700.0) - 24.0);
                    layout.min_width = px(0);
                    layout.flex_direction = FlexDirection::Row;
                    layout.flex_wrap = FlexWrap::Wrap;
                    layout.align_content = AlignContent::Center;
                    layout.justify_content = JustifyContent::Center;
                    layout.column_gap = px(10);
                    layout.row_gap = px(8);
                    layout.padding = UiRect::all(px(12));
                }
                _ => {}
            }
            if button.is_some() {
                layout.width = Val::Percent(47.0);
                layout.height = px(48);
                layout.min_height = px(48);
                layout.flex_shrink = 0.0;
            }
        }
        if *node != layout {
            *node = layout;
        }
    }
}
