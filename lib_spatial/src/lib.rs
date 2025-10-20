pub const CHUNK_SIZE: usize = 32;

pub fn pos_to_index_3d([x, y, z]: [usize; 3]) -> usize {
    z + y * CHUNK_SIZE + x * CHUNK_SIZE * CHUNK_SIZE
}

pub fn pos_to_index_2d([x, y]: [usize; 2]) -> usize {
    y + x * CHUNK_SIZE
}

pub trait SpatiallyMapped<const DIM: usize> {
    type Item;
    type Index;
    fn at_pos(&self, pos: [Self::Index; DIM]) -> &Self::Item;
}

impl<T> SpatiallyMapped<2> for Vec<T> {
    type Index = usize;
    type Item = T;
    fn at_pos(&self, pos: [usize; 2]) -> &T {
        let i = pos_to_index_2d(pos);
        return &self[i];
    }
}

impl<T> SpatiallyMapped<3> for Vec<T> {
    type Index = usize;
    type Item = T;
    fn at_pos(&self, pos: [usize; 3]) -> &T {
        let i = pos_to_index_3d(pos);
        return &self[i];
    }
}
