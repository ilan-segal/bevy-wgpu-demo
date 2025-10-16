use std::{collections::HashMap, marker::PhantomData};

use bevy::prelude::*;
use lib_spatial::{CHUNK_SIZE, SpatiallyMapped};

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
        app.add_systems(Startup, init_index::<T>)
            .add_observer(add_to_index::<T>)
            .add_observer(remove_from_index::<T>);
    }
}

fn init_index<T: Component>(mut commands: Commands) {
    let index = ChunkIndex {
        entity_by_position: HashMap::new(),
        _phantom: PhantomData::<T>,
    };
    commands.insert_resource(index);
}

fn add_to_index<T: Component>(
    trigger: Trigger<OnAdd, T>,
    query: Query<&ChunkPosition, With<T>>,
    mut index: ResMut<ChunkIndex<T>>,
) {
    let e = trigger.target();
    let Ok(chunk_pos) = query.get(e) else {
        warn!("Failed to get chunk position for entity {:?}", e);
        return;
    };
    index.entity_by_position.insert(chunk_pos.0, e);
}

fn remove_from_index<T: Component>(
    trigger: Trigger<OnRemove, T>,
    query: Query<&ChunkPosition, With<T>>,
    mut index: ResMut<ChunkIndex<T>>,
) {
    let e = trigger.target();
    let Ok(chunk_pos) = query.get(e) else {
        warn!("Failed to get chunk position for entity {:?}", e);
        return;
    };
    index.entity_by_position.remove(&chunk_pos.0);
}

pub struct Neighborhood<Chunk> {
    pub chunks: [Option<Chunk>; 9],
}

impl<Chunk> Neighborhood<Chunk> {
    pub fn get_chunk(&self, pos: &[i32; 3]) -> &Option<Chunk> {
        let [x, y, z] = pos;
        // Coords of chunk within neighborhood
        let index = (x + 1) + 3 * (y + 1) + 9 * (z + 1);
        return &self.chunks[index as usize];
    }

    pub fn get_middle(&self) -> &Chunk {
        let Some(chunk) = self.get_chunk(&[1, 1, 1]) else {
            panic!("Middle chunk not found");
        };
        return chunk;
    }
}

impl<Chunk> Neighborhood<Chunk>
where
    Chunk: SpatiallyMapped<3, Index = usize>,
{
    pub fn at_pos(&self, pos: &[i32; 3]) -> Option<&Chunk::Item> {
        const SIZE: i32 = CHUNK_SIZE as i32;
        let [x, y, z] = pos;
        /// 0, 1, 2
        fn get_neighborhood_axis_coord(axis_coord: &i32) -> usize {
            if *axis_coord < 0 {
                0
            } else if axis_coord < &(CHUNK_SIZE as i32) {
                1
            } else {
                2
            }
        }
        // Coords of chunk within neighborhood
        let xn = get_neighborhood_axis_coord(x);
        let yn = get_neighborhood_axis_coord(y);
        let zn = get_neighborhood_axis_coord(z);
        let index = xn + 3 * yn + 9 * zn;
        let Some(ref chunk) = self.chunks[index] else {
            return None;
        };
        // Chunk-local coordinates
        let xl = ((x + SIZE) % SIZE) as usize;
        let yl = ((y + SIZE) % SIZE) as usize;
        let zl = ((z + SIZE) % SIZE) as usize;
        return Some(chunk.at_pos([xl, yl, zl]));
    }
}
