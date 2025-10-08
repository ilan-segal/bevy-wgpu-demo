use bevy::{input::mouse::MouseMotion, prelude::*};
use std::{
    f32::consts::{PI, TAU},
    marker::PhantomData,
};

/// Given a marker component, this plugin will make a marked entity move with the mouse like an FPS camera.
pub struct FirstPersonCameraPlugin<CameraMarker: Component> {
    _phantom: PhantomData<CameraMarker>,
}

impl<CameraMarker: Component> FirstPersonCameraPlugin<CameraMarker> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<CameraMarker: Component> Plugin for FirstPersonCameraPlugin<CameraMarker> {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraControls>()
            .init_resource::<CameraMouseSensitivity>()
            .init_resource::<CameraSpeed>()
            .add_systems(
                Update,
                (
                    add_pitch_yaw::<CameraMarker>,
                    (
                        update_pitch_yaw::<CameraMarker>,
                        align_camera_with_pitch_yaw,
                        move_camera_from_keyboard_input::<CameraMarker>,
                    )
                        .chain(),
                ),
            );
    }
}

#[derive(Resource)]
pub struct CameraControls {
    pub forward: KeyCode,
    pub backward: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
    pub up: KeyCode,
    pub down: KeyCode,
    pub mouse_x_inverted: bool,
    pub mouse_y_inverted: bool,
    pub speed_up: KeyCode,
}

impl Default for CameraControls {
    fn default() -> Self {
        Self {
            forward: KeyCode::KeyW,
            backward: KeyCode::KeyS,
            left: KeyCode::KeyA,
            right: KeyCode::KeyD,
            up: KeyCode::Space,
            down: KeyCode::ShiftLeft,
            mouse_x_inverted: false,
            mouse_y_inverted: false,
            speed_up: KeyCode::ControlLeft,
        }
    }
}

#[derive(Resource)]
pub struct CameraMouseSensitivity {
    pub x: f32,
    pub y: f32,
}

impl Default for CameraMouseSensitivity {
    fn default() -> Self {
        Self { x: 0.005, y: 0.005 }
    }
}

#[derive(Resource)]
pub struct CameraSpeed(pub f32);

impl Default for CameraSpeed {
    fn default() -> Self {
        Self(0.1)
    }
}

#[derive(Component, Default)]
struct CameraPitchYaw {
    pitch: f32,
    yaw: f32,
}

impl CameraPitchYaw {
    fn add_pitch(&mut self, radians: f32) {
        self.pitch = (self.pitch - radians).clamp(-PI * 0.4999, PI * 0.4999);
    }

    fn add_yaw(&mut self, radians: f32) {
        self.yaw = (self.yaw - radians) % TAU;
    }
}

impl From<Quat> for CameraPitchYaw {
    fn from(value: Quat) -> Self {
        let (pitch, yaw, _) = value.to_euler(EulerRot::XYZ);
        Self { pitch, yaw }
    }
}

fn add_pitch_yaw<CameraMarker: Component>(
    mut commands: Commands,
    q_camera: Query<(Entity, &Transform), (With<CameraMarker>, Without<CameraPitchYaw>)>,
) {
    for (e, transform) in q_camera.iter() {
        commands
            .entity(e)
            .try_insert(CameraPitchYaw::from(transform.rotation));
    }
}

fn update_pitch_yaw<CameraMarker: Component>(
    mut q_camera: Query<&mut CameraPitchYaw, With<CameraMarker>>,
    mut evr_motion: EventReader<MouseMotion>,
    controls: Res<CameraControls>,
    sensitivity: Res<CameraMouseSensitivity>,
) {
    for ev in evr_motion.read() {
        let x = controls.mouse_x_inverted.then_some(-1.).unwrap_or(1.) * sensitivity.x * ev.delta.x;
        let y = controls.mouse_y_inverted.then_some(-1.).unwrap_or(1.) * sensitivity.y * ev.delta.y;
        for mut pitch_yaw in q_camera.iter_mut() {
            pitch_yaw.add_pitch(y);
            pitch_yaw.add_yaw(x);
        }
    }
}

fn align_camera_with_pitch_yaw(mut q_camera: Query<(&mut Transform, &CameraPitchYaw)>) {
    for (mut transform, pitch_yaw) in q_camera.iter_mut() {
        transform.rotation = {
            let mut t = Transform::default();
            t.rotate_x(pitch_yaw.pitch);
            t.rotate_y(pitch_yaw.yaw);
            t.rotation
        };
    }
}

fn move_camera_from_keyboard_input<CameraMarker: Component>(
    mut q_camera: Query<&mut Transform, With<CameraMarker>>,
    keys: Res<ButtonInput<KeyCode>>,
    controls: Res<CameraControls>,
    speed: Res<CameraSpeed>,
) {
    for mut transform in q_camera.iter_mut() {
        let mut d = Vec3::ZERO;
        if keys.pressed(controls.left) {
            d += transform.left().as_vec3();
        }
        if keys.pressed(controls.right) {
            d += transform.right().as_vec3();
        }
        if keys.pressed(controls.forward) {
            d += transform.forward().as_vec3().with_y(0.).normalize();
        }
        if keys.pressed(controls.backward) {
            d += transform.back().as_vec3().with_y(0.).normalize();
        }
        if keys.pressed(controls.up) {
            d += Vec3::Y;
        }
        if keys.pressed(controls.down) {
            d += Vec3::NEG_Y;
        }
        if d != Vec3::ZERO {
            d = d.normalize();
        }
        let factor = if keys.pressed(controls.speed_up) {
            10.0
        } else {
            1.0
        };
        transform.translation += d * factor * speed.0;
    }
}
