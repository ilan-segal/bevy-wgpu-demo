use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin,
    ecs::system::lifetimeless::{SQuery, SRes},
    prelude::*,
};
use iyes_perf_ui::{entry::PerfUiEntry, prelude::*};

use crate::mesh::QuadCount;

pub struct DebugHudPlugin;

impl Plugin for DebugHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((FrameTimeDiagnosticsPlugin::default(), PerfUiPlugin))
            .add_perf_ui_simple_entry::<PerfUiEntryQuadCount>()
            .add_perf_ui_simple_entry::<PerfUiEntryCameraPosition>()
            .add_perf_ui_simple_entry::<PerfUiEntryCameraForward>()
            .add_systems(Startup, spawn_perf_ui_entries);
    }
}

fn spawn_perf_ui_entries(mut commands: Commands) {
    commands.spawn((
        PerfUiEntryFPSAverage::default(),
        PerfUiEntryFPSPctLow::default(),
        PerfUiEntryFrameTime::default(),
        PerfUiEntryQuadCount::default(),
        PerfUiEntryCameraPosition::default(),
        PerfUiEntryCameraForward::default(),
    ));
}

#[derive(Component)]
#[require(PerfUiRoot)]
struct PerfUiEntryCameraForward {
    pub sort_key: i32,
}

impl Default for PerfUiEntryCameraForward {
    fn default() -> Self {
        Self {
            sort_key: iyes_perf_ui::utils::next_sort_key(),
        }
    }
}

impl PerfUiEntry for PerfUiEntryCameraForward {
    type Value = Dir3;
    type SystemParam = SQuery<&'static GlobalTransform, With<Camera3d>>;

    fn label(&self) -> &str {
        "Camera Forward"
    }

    fn sort_key(&self) -> i32 {
        self.sort_key
    }

    fn update_value(
        &self,
        param: &mut <Self::SystemParam as bevy::ecs::system::SystemParam>::Item<'_, '_>,
    ) -> Option<Self::Value> {
        param.single().map(|t| t.forward()).ok()
    }

    fn format_value(&self, value: &Self::Value) -> String {
        format!("{:.1} / {:.1} / {:.1}", value.x, value.y, value.z)
    }
}

#[derive(Component)]
#[require(PerfUiRoot)]
struct PerfUiEntryCameraPosition {
    pub sort_key: i32,
}

impl Default for PerfUiEntryCameraPosition {
    fn default() -> Self {
        Self {
            sort_key: iyes_perf_ui::utils::next_sort_key(),
        }
    }
}

impl PerfUiEntry for PerfUiEntryCameraPosition {
    type Value = Vec3;
    type SystemParam = SQuery<&'static GlobalTransform, With<Camera3d>>;

    fn label(&self) -> &str {
        "Camera Position"
    }

    fn sort_key(&self) -> i32 {
        self.sort_key
    }

    fn update_value(
        &self,
        param: &mut <Self::SystemParam as bevy::ecs::system::SystemParam>::Item<'_, '_>,
    ) -> Option<Self::Value> {
        param.single().map(|t| t.translation()).ok()
    }

    fn format_value(&self, value: &Self::Value) -> String {
        format!("{:.1} / {:.1} / {:.1}", value.x, value.y, value.z)
    }
}

#[derive(Component)]
#[require(PerfUiRoot)]
struct PerfUiEntryQuadCount {
    pub sort_key: i32,
}

impl Default for PerfUiEntryQuadCount {
    fn default() -> Self {
        Self {
            sort_key: iyes_perf_ui::utils::next_sort_key(),
        }
    }
}

impl PerfUiEntry for PerfUiEntryQuadCount {
    type Value = u32;
    type SystemParam = SRes<QuadCount>;

    fn label(&self) -> &str {
        "Quad Count"
    }

    fn sort_key(&self) -> i32 {
        self.sort_key
    }

    fn update_value(
        &self,
        param: &mut <Self::SystemParam as bevy::ecs::system::SystemParam>::Item<'_, '_>,
    ) -> Option<Self::Value> {
        Some(param.0)
    }

    fn format_value(&self, value: &Self::Value) -> String {
        format!("{}", value)
    }
}
