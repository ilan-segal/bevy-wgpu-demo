use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use bevy::prelude::*;
use lib_spatial::{CHUNK_SIZE, SpatiallyMapped};
use lib_utils::cube_iter;

#[derive(Component, Clone, Copy)]
pub struct ChunkPosition(pub IVec3);

impl From<IVec3> for ChunkPosition {
    fn from(value: IVec3) -> Self {
        Self(value)
    }
}

impl From<(i32, i32, i32)> for ChunkPosition {
    fn from(value: (i32, i32, i32)) -> Self {
        Self(IVec3::from(value))
    }
}

impl From<ChunkPosition> for IVec3 {
    fn from(value: ChunkPosition) -> Self {
        value.0
    }
}

#[derive(Resource, Default)]
pub struct ChunkIndex {
    entity_by_position: HashMap<IVec3, Entity>,
    position_by_entity: HashMap<Entity, IVec3>,
}

impl ChunkIndex {
    pub fn get_entity(&self, pos: &IVec3) -> Option<&Entity> {
        self.entity_by_position.get(pos)
    }

    pub fn get_position(&self, e: &Entity) -> Option<&IVec3> {
        self.position_by_entity.get(e)
    }
}

pub struct ChunkIndexPlugin;

impl Plugin for ChunkIndexPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkIndex>()
            .add_observer(add_to_index)
            .add_observer(remove_from_index);
    }
}

fn add_to_index(
    trigger: Trigger<OnAdd, ChunkPosition>,
    query: Query<&ChunkPosition>,
    mut index: ResMut<ChunkIndex>,
) {
    let e = trigger.target();
    let Ok(chunk_pos) = query.get(e) else {
        warn!("Failed to get chunk position for entity {:?}", e);
        return;
    };
    index.entity_by_position.insert(chunk_pos.0, e);
    index.position_by_entity.insert(e, chunk_pos.0);
}

fn remove_from_index(
    trigger: Trigger<OnRemove, ChunkPosition>,
    query: Query<&ChunkPosition>,
    mut index: ResMut<ChunkIndex>,
) {
    let e = trigger.target();
    let Ok(chunk_pos) = query.get(e) else {
        warn!("Failed to get chunk position for entity {:?}", e);
        return;
    };
    index.entity_by_position.remove(&chunk_pos.0);
    index.position_by_entity.remove(&e);
}

pub struct NeighborhoodPlugin<T: Component> {
    _phantom: PhantomData<T>,
}

impl<T: Component> NeighborhoodPlugin<T> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T: Component + Clone> Plugin for NeighborhoodPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_event::<NeighborUpdateEvent<T>>()
            .add_event::<NewNeighborhood<T>>()
            .add_systems(
                Update,
                (
                    update_component_copy::<T>,
                    (
                        emit_event_for_adding_neighborhood::<T>,
                        populate_neighborhood::<T>,
                    )
                        .chain(),
                    (
                        emit_update_event_with_changed_neighbor::<T>,
                        consume_neighbor_update_events::<T>,
                    )
                        .chain(),
                    assign_full_neighborhood::<T>,
                    revoke_full_neighborhood::<T>,
                ),
            )
            .add_observer(notify_neighbors_on_delete::<T>);
    }
}

#[derive(Component, Clone)]
struct ComponentCopy<T: Component + Clone> {
    value: Arc<T>,
}

fn update_component_copy<T: Component + Clone>(
    mut commands: Commands,
    q: Query<(Entity, &T), Changed<T>>,
) {
    for (entity, component) in q.iter() {
        let value = Arc::new(component.clone());
        let copy = ComponentCopy { value };
        commands.entity(entity).try_insert(copy);
    }
}

#[derive(Component, Default, Clone)]
pub struct Neighborhood<T> {
    pub chunks: [Option<Arc<T>>; 27],
}

impl<T> Neighborhood<T> {
    /// pos ∈ {-1, 0, 1}^3
    pub fn get_chunk(&self, pos: &[i32; 3]) -> &Option<Arc<T>> {
        let [x, y, z] = pos;
        // Coords of chunk within neighborhood
        let index = (x + 1) + 3 * (y + 1) + 9 * (z + 1);
        return &self.chunks[index as usize];
    }
    /// pos ∈ {-1, 0, 1}^3
    pub fn put_chunk(&mut self, pos: &[i32; 3], value: Option<Arc<T>>) {
        let [x, y, z] = pos;
        // Coords of chunk within neighborhood
        let index = (x + 1) + 3 * (y + 1) + 9 * (z + 1);
        self.chunks[index as usize] = value;
    }

    pub fn get_middle(&self) -> &T {
        let Some(chunk) = self.get_chunk(&[1, 1, 1]) else {
            panic!("Middle chunk not found");
        };
        return chunk;
    }
}

impl<T> Neighborhood<T>
where
    T: SpatiallyMapped<3, Index = usize>,
{
    pub fn at_pos(&self, pos: &[i32; 3]) -> Option<&T::Item> {
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
        // Chunk-local coordinates
        let xl = ((x + SIZE) % SIZE) as usize;
        let yl = ((y + SIZE) % SIZE) as usize;
        let zl = ((z + SIZE) % SIZE) as usize;
        let index = xn + 3 * yn + 9 * zn;
        return self.chunks[index].as_ref().map(|c| c.at_pos([xl, yl, zl]));
    }
}

#[derive(Event)]
struct NewNeighborhood<T: Component> {
    entity: Entity,
    position: IVec3,
    _phantom: PhantomData<T>,
}

fn emit_event_for_adding_neighborhood<T: Component + Clone>(
    q: Query<(Entity, &ChunkPosition), (With<ComponentCopy<T>>, Without<Neighborhood<T>>)>,
    mut ew: EventWriter<NewNeighborhood<T>>,
) {
    for (entity, pos) in q.iter() {
        let position = pos.0;
        let event = NewNeighborhood {
            position,
            entity,
            _phantom: PhantomData,
        };
        ew.write(event);
    }
}

fn populate_neighborhood<T: Component + Clone>(
    mut er: EventReader<NewNeighborhood<T>>,
    mut commands: Commands,
    chunk_index: Res<ChunkIndex>,
    q: Query<&ComponentCopy<T>>,
) {
    for NewNeighborhood {
        entity, position, ..
    } in er.read()
    {
        let mut neighborhood = Neighborhood::<T> {
            chunks: [const { None }; 27],
        };
        for (x, y, z) in cube_iter(-1..=1) {
            let offset = IVec3::new(x, y, z);
            let neighbor_pos = position + offset;
            let Some(neighbor_entity) = chunk_index.get_entity(&neighbor_pos) else {
                continue;
            };
            let Ok(neighbor) = q.get(*neighbor_entity) else {
                continue;
            };
            neighborhood.put_chunk(&offset.to_array(), Some(neighbor.value.clone()));
        }
        commands.entity(*entity).try_insert(neighborhood);
    }
}

#[derive(Event)]
struct NeighborUpdateEvent<T: Component + Clone> {
    pos: ChunkPosition,
    value: Option<ComponentCopy<T>>,
}

fn emit_update_event_with_changed_neighbor<T: Component + Clone>(
    q_changed: Query<(&ChunkPosition, &ComponentCopy<T>), Changed<ComponentCopy<T>>>,
    mut ew: EventWriter<NeighborUpdateEvent<T>>,
) {
    for (pos, value) in q_changed.iter() {
        let pos = *pos;
        let value = Some(value.clone());
        let event = NeighborUpdateEvent { pos, value };
        ew.write(event);
    }
}

fn notify_neighbors_on_delete<T: Component + Clone>(
    trigger: Trigger<OnRemove, ComponentCopy<T>>,
    mut ew: EventWriter<NeighborUpdateEvent<T>>,
    q: Query<&ChunkPosition>,
) {
    let entity = trigger.target();
    let Ok(pos) = q.get(entity).cloned() else {
        warn!(
            "Could not get position for notifying neighborhood of deletion: {:?}",
            entity
        );
        return;
    };
    let event = NeighborUpdateEvent { pos, value: None };
    ew.write(event);
}

fn consume_neighbor_update_events<T: Component + Clone>(
    mut er: EventReader<NeighborUpdateEvent<T>>,
    chunk_index: Res<ChunkIndex>,
    mut q_neighborhood: Query<&mut Neighborhood<T>>,
) {
    for event in er.read() {
        let center = event.pos;
        let value = event.value.as_ref().map(|x| x.value.clone());
        for (x, y, z) in cube_iter(-1..=1) {
            let offset = IVec3::new(x, y, z);
            let pos = center.0 + offset;
            let Some(entity) = chunk_index.get_entity(&pos) else {
                continue;
            };
            let Ok(ref mut neighborhood) = q_neighborhood.get_mut(*entity) else {
                continue;
            };
            let flipped_offset = offset * -1;
            let neighborhood_chunk_pos = flipped_offset.to_array();
            neighborhood.put_chunk(&neighborhood_chunk_pos, value.clone());
        }
    }
}

#[derive(Component)]
pub struct FullNeighborhood<T> {
    pub chunks: [Arc<T>; 27],
}

impl<T> FullNeighborhood<T> {
    /// pos ∈ {-1, 0, 1}^3
    pub fn get_chunk(&self, pos: &[i32; 3]) -> &Arc<T> {
        let [x, y, z] = pos;
        // Coords of chunk within neighborhood
        let index = (x + 1) + 3 * (y + 1) + 9 * (z + 1);
        return &self.chunks[index as usize];
    }

    pub fn get_middle(&self) -> &Arc<T> {
        return &self.get_chunk(&[1, 1, 1]);
    }
}

impl<T> FullNeighborhood<T>
where
    T: SpatiallyMapped<3, Index = usize>,
{
    pub fn at_pos(&self, pos: &[i32; 3]) -> &T::Item {
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
        // Chunk-local coordinates
        let xl = ((x + SIZE) % SIZE) as usize;
        let yl = ((y + SIZE) % SIZE) as usize;
        let zl = ((z + SIZE) % SIZE) as usize;
        let index = xn + 3 * yn + 9 * zn;
        return self.chunks[index].at_pos([xl, yl, zl]);
    }
}

fn assign_full_neighborhood<T: Component>(
    mut commands: Commands,
    q_neighborhood: Query<(Entity, &Neighborhood<T>), Changed<Neighborhood<T>>>,
) {
    for (entity, neighborhood) in q_neighborhood.iter() {
        if neighborhood.chunks.iter().any(Option::is_none) {
            continue;
        }
        let chunks = neighborhood.chunks.clone().map(Option::unwrap);
        let full_neighborhood = FullNeighborhood { chunks };
        commands.entity(entity).try_insert(full_neighborhood);
    }
}

fn revoke_full_neighborhood<T: Component>(
    mut commands: Commands,
    q_neighborhood: Query<
        (Entity, &Neighborhood<T>),
        (Changed<Neighborhood<T>>, With<FullNeighborhood<T>>),
    >,
) {
    for (entity, neighborhood) in q_neighborhood.iter() {
        if neighborhood.chunks.iter().all(Option::is_some) {
            continue;
        }
        commands.entity(entity).try_remove::<FullNeighborhood<T>>();
    }
}
