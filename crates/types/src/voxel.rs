use crate::genome::Genome;

/// Voxel types matching WGSL constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VoxelType {
    Empty = 0,
    Wall = 1,
    Nutrient = 2,
    EnergySource = 3,
    Protocell = 4,
    Waste = 5,
    HeatSource = 6,
    ColdSource = 7,
}

impl VoxelType {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Empty,
            1 => Self::Wall,
            2 => Self::Nutrient,
            3 => Self::EnergySource,
            4 => Self::Protocell,
            5 => Self::Waste,
            6 => Self::HeatSource,
            7 => Self::ColdSource,
            _ => Self::Empty,
        }
    }
}

/// A single voxel: 32 bytes = 8 × u32.
///
/// Word 0: [0:7] type  [8:15] flags  [16:31] energy (u16)
/// Word 1: [0:15] age (u16)  [16:31] species_id (u16)
/// Words 2-5: genome (16 bytes, 4 × u32)
/// Words 6-7: extra (type-specific state)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Voxel {
    pub voxel_type: VoxelType,
    pub flags: u8,
    pub energy: u16,
    pub age: u16,
    pub species_id: u16,
    pub genome: Genome,
    pub extra: [u32; 2],
}

impl Default for Voxel {
    fn default() -> Self {
        Self {
            voxel_type: VoxelType::Empty,
            flags: 0,
            energy: 0,
            age: 0,
            species_id: 0,
            genome: Genome::default(),
            extra: [0; 2],
        }
    }
}

impl Voxel {
    /// Pack voxel into 8 u32 words matching the GPU buffer layout.
    pub fn pack(&self) -> [u32; 8] {
        let mut words = [0u32; 8];
        // Word 0: [0:7] type | [8:15] flags | [16:31] energy
        words[0] = (self.voxel_type as u32)
            | ((self.flags as u32) << 8)
            | ((self.energy as u32) << 16);
        // Word 1: [0:15] age | [16:31] species_id
        words[1] = (self.age as u32)
            | ((self.species_id as u32) << 16);
        // Words 2-5: genome
        let gw = self.genome.to_words();
        words[2] = gw[0];
        words[3] = gw[1];
        words[4] = gw[2];
        words[5] = gw[3];
        // Words 6-7: extra
        words[6] = self.extra[0];
        words[7] = self.extra[1];
        words
    }

    /// Unpack voxel from 8 u32 words.
    pub fn unpack(words: [u32; 8]) -> Self {
        let voxel_type = VoxelType::from_u8((words[0] & 0xFF) as u8);
        let flags = ((words[0] >> 8) & 0xFF) as u8;
        let energy = ((words[0] >> 16) & 0xFFFF) as u16;
        let age = (words[1] & 0xFFFF) as u16;
        let species_id = ((words[1] >> 16) & 0xFFFF) as u16;
        let genome = Genome::from_words([words[2], words[3], words[4], words[5]]);
        let extra = [words[6], words[7]];
        Self {
            voxel_type,
            flags,
            energy,
            age,
            species_id,
            genome,
            extra,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_empty() {
        let v = Voxel::default();
        let packed = v.pack();
        let v2 = Voxel::unpack(packed);
        assert_eq!(v, v2);
    }

    #[test]
    fn roundtrip_protocell() {
        let v = Voxel {
            voxel_type: VoxelType::Protocell,
            flags: 0x55,
            energy: 1000,
            age: 42,
            species_id: 12345,
            genome: Genome { bytes: [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 0, 0, 0, 0, 0] },
            extra: [0xDEAD, 0xBEEF],
        };
        let packed = v.pack();
        let v2 = Voxel::unpack(packed);
        assert_eq!(v, v2);
    }

    #[test]
    fn roundtrip_max_values() {
        let v = Voxel {
            voxel_type: VoxelType::ColdSource,
            flags: 0xFF,
            energy: 0xFFFF,
            age: 0xFFFF,
            species_id: 0xFFFF,
            genome: Genome { bytes: [0xFF; 16] },
            extra: [0xFFFFFFFF, 0xFFFFFFFF],
        };
        let packed = v.pack();
        let v2 = Voxel::unpack(packed);
        assert_eq!(v, v2);
    }

    #[test]
    fn word_layout_matches_spec() {
        let v = Voxel {
            voxel_type: VoxelType::Protocell, // 4
            flags: 0xAB,
            energy: 0x1234,
            age: 0x5678,
            species_id: 0x9ABC,
            genome: Genome::default(),
            extra: [0; 2],
        };
        let words = v.pack();

        // Word 0: [0:7]=type=4, [8:15]=flags=0xAB, [16:31]=energy=0x1234
        assert_eq!(words[0] & 0xFF, 4);
        assert_eq!((words[0] >> 8) & 0xFF, 0xAB);
        assert_eq!((words[0] >> 16) & 0xFFFF, 0x1234);

        // Word 1: [0:15]=age=0x5678, [16:31]=species_id=0x9ABC
        assert_eq!(words[1] & 0xFFFF, 0x5678);
        assert_eq!((words[1] >> 16) & 0xFFFF, 0x9ABC);
    }

    #[test]
    fn voxel_type_from_u8_valid() {
        assert_eq!(VoxelType::from_u8(0), VoxelType::Empty);
        assert_eq!(VoxelType::from_u8(4), VoxelType::Protocell);
        assert_eq!(VoxelType::from_u8(7), VoxelType::ColdSource);
    }

    #[test]
    fn voxel_type_from_u8_invalid_defaults_empty() {
        assert_eq!(VoxelType::from_u8(8), VoxelType::Empty);
        assert_eq!(VoxelType::from_u8(255), VoxelType::Empty);
    }

    #[test]
    fn pack_energy_boundaries() {
        for energy in [0u16, 1, 65534, 65535] {
            let v = Voxel {
                voxel_type: VoxelType::Protocell,
                energy,
                ..Default::default()
            };
            let v2 = Voxel::unpack(v.pack());
            assert_eq!(v2.energy, energy, "energy {energy} not preserved");
        }
    }

    #[test]
    fn pack_genome_all_bytes() {
        let mut genome = Genome::default();
        for i in 0u8..16 {
            genome.bytes[i as usize] = i * 17;
        }
        let v = Voxel {
            voxel_type: VoxelType::Protocell,
            genome,
            ..Default::default()
        };
        let v2 = Voxel::unpack(v.pack());
        for i in 0..16 {
            assert_eq!(v2.genome.bytes[i], (i as u8) * 17, "genome byte {i} mismatch");
        }
    }
}
