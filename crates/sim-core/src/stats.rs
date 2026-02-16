/// Stats readback data parsed from the 128-byte stats_buf.
/// Layout: 32 × u32 words.
///   [0] population
///   [1] total_energy
///   [2] species_count (unused — derived from histogram)
///   [3] max_energy
///   [4..27] species histogram: 12 entries × 2 words (species_id, count)
///   [28..31] reserved
#[derive(Debug, Clone, Default)]
pub struct SimStats {
    pub population: u32,
    pub total_energy: u32,
    pub species_count: u32,
    pub max_energy: u32,
    pub species_histogram: Vec<(u16, u32)>,
}

impl SimStats {
    pub fn from_words(words: &[u32; 32]) -> Self {
        let population = words[0];
        let total_energy = words[1];
        let max_energy = words[3];

        let mut species_histogram = Vec::new();
        for i in 0..12 {
            let sid = words[4 + i * 2] as u16;
            let count = words[5 + i * 2];
            if sid != 0 && count > 0 {
                species_histogram.push((sid, count));
            }
        }
        species_histogram.sort_by(|a, b| b.1.cmp(&a.1));

        let species_count = species_histogram.len() as u32;

        SimStats {
            population,
            total_energy,
            species_count,
            max_energy,
            species_histogram,
        }
    }
}
