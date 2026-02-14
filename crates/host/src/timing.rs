pub struct FrameTiming {
    pub frame_count: u64,
    pub last_dt: f32,
}

impl FrameTiming {
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            last_dt: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.frame_count += 1;
        self.last_dt = dt;
    }
}
