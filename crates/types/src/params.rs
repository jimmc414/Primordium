/// Simulation parameters. All f32 for uniform buffer compatibility.
/// Serialized to bytes and uploaded as a GPU uniform buffer.
#[derive(Debug, Clone)]
pub struct SimParams {
    pub grid_size: f32,
    pub tick_count: f32,
    pub dt: f32,
    pub nutrient_spawn_rate: f32,
    pub waste_decay_ticks: f32,
    pub nutrient_recycle_rate: f32,
    pub movement_energy_cost: f32,
    pub base_ambient_temp: f32,
    pub metabolic_cost_base: f32,
    pub replication_energy_min: f32,
    pub energy_from_nutrient: f32,
    pub energy_from_source: f32,
    pub diffusion_rate: f32,
    pub temp_sensitivity: f32,
    pub predation_energy_fraction: f32,
    pub max_energy: f32,
    pub overlay_mode: f32,   // 0.0=normal, 1.0=temperature
    pub _pad17: f32,
    pub _pad18: f32,
    pub _pad19: f32,
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            grid_size: 128.0,
            tick_count: 0.0,
            dt: 0.016,
            nutrient_spawn_rate: 0.001,
            waste_decay_ticks: 100.0,
            nutrient_recycle_rate: 0.5,
            movement_energy_cost: 5.0,
            base_ambient_temp: 0.5,
            metabolic_cost_base: 2.0,
            replication_energy_min: 200.0,
            energy_from_nutrient: 50.0,
            energy_from_source: 10.0,
            diffusion_rate: 0.1,
            temp_sensitivity: 1.0,
            predation_energy_fraction: 0.5,
            max_energy: 1000.0,
            overlay_mode: 0.0,
            _pad17: 0.0,
            _pad18: 0.0,
            _pad19: 0.0,
        }
    }
}

impl SimParams {
    /// Serialize all fields to bytes, padded to 16-byte alignment.
    pub fn to_bytes(&self) -> Vec<u8> {
        let fields: [f32; 20] = [
            self.grid_size,
            self.tick_count,
            self.dt,
            self.nutrient_spawn_rate,
            self.waste_decay_ticks,
            self.nutrient_recycle_rate,
            self.movement_energy_cost,
            self.base_ambient_temp,
            self.metabolic_cost_base,
            self.replication_energy_min,
            self.energy_from_nutrient,
            self.energy_from_source,
            self.diffusion_rate,
            self.temp_sensitivity,
            self.predation_energy_fraction,
            self.max_energy,
            self.overlay_mode,
            self._pad17,
            self._pad18,
            self._pad19,
        ];
        let mut bytes = Vec::with_capacity(fields.len() * 4);
        for f in &fields {
            bytes.extend_from_slice(&f.to_le_bytes());
        }
        // 80 bytes = 20 fields * 4 bytes, which is 16-byte aligned
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_bytes_length_aligned() {
        let p = SimParams::default();
        let bytes = p.to_bytes();
        assert_eq!(bytes.len(), 80); // 20 fields * 4 bytes
        assert_eq!(bytes.len() % 16, 0, "must be 16-byte aligned");
    }

    #[test]
    fn to_bytes_roundtrip_grid_size() {
        let p = SimParams { grid_size: 64.0, ..Default::default() };
        let bytes = p.to_bytes();
        let val = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(val, 64.0);
    }
}
