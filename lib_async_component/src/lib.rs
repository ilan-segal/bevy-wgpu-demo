use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use bevy::{
    ecs::world::OnDespawn,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, block_on, futures_lite::future},
};

pub struct AsyncComponentPlugin<T> {
    _phantom: PhantomData<T>,
}

impl<T: Component> AsyncComponentPlugin<T> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T: Component> Plugin for AsyncComponentPlugin<T> {
    fn build(&self, app: &mut App) {
        app.insert_resource(ComputeTasks::<T> {
            tasks: HashMap::new(),
            added_since_last_update: HashSet::new(),
        })
        .add_systems(
            PostUpdate,
            (
                update_compute_in_progress_flags::<T>,
                recieve_compute_tasks::<T>,
            )
                .chain(),
        )
        .add_observer(kill_compute_task::<T>);
    }
}

#[derive(Resource)]
pub struct ComputeTasks<T> {
    tasks: HashMap<Entity, Task<T>>,
    added_since_last_update: HashSet<Entity>,
}

#[derive(Component)]
pub struct ComputeInProgress<T> {
    _phantom: PhantomData<T>,
}

impl<T: Send + 'static> ComputeTasks<T> {
    pub fn spawn_task<Future: std::future::Future<Output = T> + Send + 'static>(
        &mut self,
        entity: Entity,
        future: Future,
    ) {
        let pool = AsyncComputeTaskPool::get();
        let task = pool.spawn(future);
        self.tasks.insert(entity, task);
        self.added_since_last_update.insert(entity);
    }
}

fn update_compute_in_progress_flags<T: Component>(
    mut commands: Commands,
    mut tasks: ResMut<ComputeTasks<T>>,
) {
    for entity in tasks.added_since_last_update.drain() {
        commands.entity(entity).try_insert(ComputeInProgress {
            _phantom: PhantomData::<T>,
        });
    }
}

fn recieve_compute_tasks<T: Component>(mut commands: Commands, mut tasks: ResMut<ComputeTasks<T>>) {
    tasks.tasks.retain(|entity, task| {
        let Some(result) = block_on(future::poll_once(task)) else {
            return true;
        };
        commands
            .entity(*entity)
            .try_insert(result)
            .try_remove::<ComputeInProgress<T>>();
        return false;
    });
}

fn kill_compute_task<T: Component>(
    trigger: Trigger<OnDespawn>,
    mut tasks: ResMut<ComputeTasks<T>>,
) {
    let entity = trigger.target();
    tasks.tasks.remove(&entity);
}
