use std::{collections::HashMap, marker::PhantomData};

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
        app.insert_resource(ComputeTasks {
            tasks: HashMap::<Entity, Task<T>>::new(),
        })
        .add_systems(Update, recieve_compute_tasks::<T>)
        .add_observer(kill_compute_task::<T>);
    }
}

#[derive(Resource)]
pub struct ComputeTasks<T> {
    tasks: HashMap<Entity, Task<T>>,
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
    }
}

fn recieve_compute_tasks<T: Component>(mut commands: Commands, mut tasks: ResMut<ComputeTasks<T>>) {
    tasks.tasks.retain(|entity, task| {
        let Some(result) = block_on(future::poll_once(task)) else {
            return true;
        };
        commands.entity(*entity).try_insert(result);
        return false;
    });
}

fn kill_compute_task<T: Send + 'static>(
    trigger: Trigger<OnDespawn>,
    mut tasks: ResMut<ComputeTasks<T>>,
) {
    let entity = trigger.target();
    tasks.tasks.remove(&entity);
}
