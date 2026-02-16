/// 16-byte genome packed into 4 × u32.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Genome {
    pub bytes: [u8; 16],
}

impl Default for Genome {
    fn default() -> Self {
        Self { bytes: [0; 16] }
    }
}

impl Genome {
    // Byte accessors matching genome byte map
    pub fn metabolic_efficiency(&self) -> u8 { self.bytes[0] }
    pub fn metabolic_rate(&self) -> u8 { self.bytes[1] }
    pub fn replication_threshold(&self) -> u8 { self.bytes[2] }
    pub fn mutation_rate(&self) -> u8 { self.bytes[3] }
    pub fn movement_bias(&self) -> u8 { self.bytes[4] }
    pub fn chemotaxis_strength(&self) -> u8 { self.bytes[5] }
    pub fn toxin_resistance(&self) -> u8 { self.bytes[6] }
    pub fn predation_capability(&self) -> u8 { self.bytes[7] }
    pub fn predation_aggression(&self) -> u8 { self.bytes[8] }
    pub fn photosynthetic_rate(&self) -> u8 { self.bytes[9] }
    pub fn energy_split_ratio(&self) -> u8 { self.bytes[10] }

    /// Pack genome into 4 u32 words (little-endian byte order).
    pub fn to_words(&self) -> [u32; 4] {
        let mut words = [0u32; 4];
        for i in 0..4 {
            let base = i * 4;
            words[i] = (self.bytes[base] as u32)
                | ((self.bytes[base + 1] as u32) << 8)
                | ((self.bytes[base + 2] as u32) << 16)
                | ((self.bytes[base + 3] as u32) << 24);
        }
        words
    }

    /// Unpack genome from 4 u32 words (little-endian byte order).
    pub fn from_words(words: [u32; 4]) -> Self {
        let mut bytes = [0u8; 16];
        for i in 0..4 {
            let base = i * 4;
            bytes[base] = (words[i] & 0xFF) as u8;
            bytes[base + 1] = ((words[i] >> 8) & 0xFF) as u8;
            bytes[base + 2] = ((words[i] >> 16) & 0xFF) as u8;
            bytes[base + 3] = ((words[i] >> 24) & 0xFF) as u8;
        }
        Self { bytes }
    }

    /// Compute species ID from genome. XOR all 4 words, then hash to u16.
    /// If result is 0, return 1 (0 is reserved for non-protocells).
    pub fn species_id(&self) -> u16 {
        let words = self.to_words();
        let mut x = words[0] ^ words[1] ^ words[2] ^ words[3];
        // hash16: three rounds of multiply-xor-shift
        x = ((x >> 8) ^ x).wrapping_mul(0x6979);
        x = ((x >> 8) ^ x).wrapping_mul(0x0235);
        x = (x >> 16) ^ x;
        let id = (x & 0xFFFF) as u16;
        if id == 0 { 1 } else { id }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genome_roundtrip_words() {
        let g = Genome { bytes: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16] };
        let words = g.to_words();
        let g2 = Genome::from_words(words);
        assert_eq!(g, g2);
    }

    #[test]
    fn species_id_nonzero() {
        // Even for an all-zero genome, species_id must not be 0
        let g = Genome::default();
        assert_ne!(g.species_id(), 0);
    }

    #[test]
    fn genome_accessors() {
        let mut g = Genome::default();
        g.bytes[0] = 42;
        g.bytes[10] = 200;
        assert_eq!(g.metabolic_efficiency(), 42);
        assert_eq!(g.energy_split_ratio(), 200);
    }

    #[test]
    fn species_id_deterministic() {
        let g = Genome { bytes: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16] };
        assert_eq!(g.species_id(), 30752);
    }

    #[test]
    fn species_hash_sensitivity() {
        let mut changed = 0u32;
        for i in 0..100u32 {
            let mut bytes_a = [0u8; 16];
            let mut bytes_b = [0u8; 16];
            for j in 0..16 {
                bytes_a[j] = ((i * 7 + j as u32 * 13) & 0xFF) as u8;
                bytes_b[j] = bytes_a[j];
            }
            // Flip one bit in a pseudo-random byte
            let byte_idx = (i as usize) % 16;
            let bit_idx = (i as usize / 16) % 8;
            bytes_b[byte_idx] ^= 1 << bit_idx;
            let ga = Genome { bytes: bytes_a };
            let gb = Genome { bytes: bytes_b };
            if ga.species_id() != gb.species_id() {
                changed += 1;
            }
        }
        assert!(changed >= 90, "only {changed}/100 single-bit flips changed species_id");
    }
}
