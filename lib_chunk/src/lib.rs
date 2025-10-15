use std::{collections::HashMap, marker::PhantomData};

use bevy::prelude::*;

#[derive(Component)]
pub struct ChunkPosition(pub IVec3);

impl From<IVec3> for ChunkPosition {
    fn from(value: IVec3) -> Self {
        Self(value)
    }
}

impl From<ChunkPosition> for IVec3 {
    fn from(value: ChunkPosition) -> Self {
        value.0
    }
}

#[derive(Resource)]
pub struct ChunkIndex<T: Component> {
    entity_by_position: HashMap<IVec3, Entity>,
    _phantom: PhantomData<T>,
}

impl<T: Component> ChunkIndex<T> {
    pub fn get_entity(&self, pos: &IVec3) -> Option<&Entity> {
        self.entity_by_position.get(pos)
    }
}

pub struct ChunkIndexPlugin<T: Component> {
    _phantom: PhantomData<T>,
}

impl<T: Component> Plugin for ChunkIndexPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_index::<T>);
    }
}

fn init_index<T: Component>(mut commands: Commands) {
    let index: ChunkIndex<T> = ChunkIndex {
        entity_by_position: HashMap::new(),
        _phantom: PhantomData,
    };
    commands.insert_resource(index);
}
