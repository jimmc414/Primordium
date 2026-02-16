/// Convert 3D coordinates to linear buffer index.
/// Formula: z * grid_size * grid_size + y * grid_size + x
#[inline]
pub fn grid_index(x: u32, y: u32, z: u32, grid_size: u32) -> usize {
    (z * grid_size * grid_size + y * grid_size + x) as usize
}

/// Convert linear buffer index back to 3D coordinates.
#[inline]
pub fn grid_coords(index: usize, grid_size: u32) -> (u32, u32, u32) {
    let index = index as u32;
    let gs = grid_size;
    let x = index % gs;
    let y = (index / gs) % gs;
    let z = index / (gs * gs);
    (x, y, z)
}

/// Von Neumann neighborhood: 6 face-adjacent offsets (±X, ±Y, ±Z).
#[inline]
pub fn neighbor_offsets() -> [(i32, i32, i32); 6] {
    [
        ( 1,  0,  0),
        (-1,  0,  0),
        ( 0,  1,  0),
        ( 0, -1,  0),
        ( 0,  0,  1),
        ( 0,  0, -1),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_index_origin() {
        assert_eq!(grid_index(0, 0, 0, 128), 0);
    }

    #[test]
    fn grid_index_last() {
        assert_eq!(grid_index(127, 127, 127, 128), 128 * 128 * 128 - 1);
    }

    #[test]
    fn grid_roundtrip() {
        let gs = 128;
        for &(x, y, z) in &[(0, 0, 0), (1, 2, 3), (63, 64, 65), (127, 127, 127)] {
            let idx = grid_index(x, y, z, gs);
            let (rx, ry, rz) = grid_coords(idx, gs);
            assert_eq!((rx, ry, rz), (x, y, z), "roundtrip failed for ({x},{y},{z})");
        }
    }

    #[test]
    fn neighbor_offsets_count() {
        assert_eq!(neighbor_offsets().len(), 6);
    }

    #[test]
    fn neighbor_offsets_symmetry() {
        let offsets = neighbor_offsets();
        for (dx, dy, dz) in &offsets {
            let neg = (-dx, -dy, -dz);
            assert!(
                offsets.contains(&neg),
                "offset ({dx},{dy},{dz}) has no negation in list"
            );
        }
    }
}
