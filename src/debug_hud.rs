use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, ecs::system::lifetimeless::SQuery, prelude::*};
use iyes_perf_ui::{entry::PerfUiEntry, prelude::*};

pub struct DebugHudPlugin;

impl Plugin for DebugHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((FrameTimeDiagnosticsPlugin::default(), PerfUiPlugin))
            .add_perf_ui_simple_entry::<PerfUiEntryCameraForward>()
            .add_systems(Startup, spawn_perf_ui_entries);
    }
}

fn spawn_perf_ui_entries(mut commands: Commands) {
    commands.spawn((
        PerfUiEntryFPSAverage::default(),
        PerfUiEntryFPSPctLow::default(),
        PerfUiEntryFrameTime::default(),
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
