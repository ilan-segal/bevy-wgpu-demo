use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use iyes_perf_ui::prelude::*;

pub struct DebugHudPlugin;

impl Plugin for DebugHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((FrameTimeDiagnosticsPlugin::default(), PerfUiPlugin))
            .add_systems(Startup, spawn_perf_ui_entries);
    }
}

fn spawn_perf_ui_entries(mut commands: Commands) {
    commands.spawn((
        PerfUiEntryFPSAverage::default(),
        PerfUiEntryFPSPctLow::default(),
        PerfUiEntryFrameTime::default(),
    ));
}
