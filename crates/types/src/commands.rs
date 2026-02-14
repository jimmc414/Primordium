/// Player command encoding for GPU upload.
/// Each command is 64 bytes = 16 × u32 words.

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    Noop = 0,
    PlaceVoxel = 1,      // param_0 = voxel_type
    RemoveVoxel = 2,
    SeedProtocells = 3,   // param_0 = initial_energy
    ApplyToxin = 4,       // param_0 = toxin_strength (0-255)
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Command {
    pub command_type: u32,
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub radius: u32,
    pub param_0: u32,
    pub param_1: u32,
    _padding: [u32; 9],
}

impl Command {
    pub fn new(command_type: CommandType, x: u32, y: u32, z: u32, radius: u32, param_0: u32, param_1: u32) -> Self {
        Self {
            command_type: command_type as u32,
            x,
            y,
            z,
            radius,
            param_0,
            param_1,
            _padding: [0u32; 9],
        }
    }

    pub fn to_words(&self) -> [u32; 16] {
        let mut words = [0u32; 16];
        words[0] = self.command_type;
        words[1] = self.x;
        words[2] = self.y;
        words[3] = self.z;
        words[4] = self.radius;
        words[5] = self.param_0;
        words[6] = self.param_1;
        // words[7..16] = padding (already zero)
        words
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_size_is_64_bytes() {
        assert_eq!(std::mem::size_of::<Command>(), 64);
    }

    #[test]
    fn command_roundtrip_words() {
        let cmd = Command::new(CommandType::PlaceVoxel, 10, 20, 30, 2, 1, 0);
        let words = cmd.to_words();
        assert_eq!(words[0], CommandType::PlaceVoxel as u32);
        assert_eq!(words[1], 10);
        assert_eq!(words[2], 20);
        assert_eq!(words[3], 30);
        assert_eq!(words[4], 2);
        assert_eq!(words[5], 1);
        assert_eq!(words[6], 0);
        for i in 7..16 {
            assert_eq!(words[i], 0, "padding word {} should be 0", i);
        }
    }
}
