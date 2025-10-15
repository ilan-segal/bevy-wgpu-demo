pub const CHUNK_SIZE: usize = 32;

pub fn pos_to_index_3d([x, y, z]: [usize; 3]) -> usize {
    x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE
}

pub fn pos_to_index_2d([x, y]: [usize; 2]) -> usize {
    x + y * CHUNK_SIZE
}

pub trait SpatiallyMapped<const DIM: usize> {
    type Item;
    type Index;
    fn at_pos(&self, pos: [Self::Index; DIM]) -> &Self::Item;
}

impl<T> SpatiallyMapped<3> for Vec<T> {
    type Index = usize;
    type Item = T;
    fn at_pos(&self, pos: [usize; 3]) -> &T {
        let i = pos_to_index_3d(pos);
        return &self[i];
    }
}

pub struct Neighborhood<Chunk> {
    pub chunks: [Option<Chunk>; 9],
}

impl<Chunk> Neighborhood<Chunk>
where
    Chunk: SpatiallyMapped<3, Index = usize>,
{
    pub fn at_pos(&self, pos: [i32; 3]) -> Option<&Chunk::Item> {
        const SIZE: i32 = CHUNK_SIZE as i32;
        let [x, y, z] = pos;
        /// 0, 1, 2
        fn get_neighborhood_axis_coord(axis_coord: i32) -> u32 {
            if axis_coord < 0 {
                0
            } else if axis_coord < CHUNK_SIZE as i32 {
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
        let Some(ref chunk) = self.chunks[index as usize] else {
            return None;
        };
        // Chunk-local coordinates
        let xl = ((x + SIZE) % SIZE) as usize;
        let yl = ((y + SIZE) % SIZE) as usize;
        let zl = ((z + SIZE) % SIZE) as usize;
        return Some(chunk.at_pos([xl, yl, zl]));
    }
}
