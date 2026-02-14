pub struct FrameTiming {
    pub frame_count: u64,
    pub last_dt: f32,
    pub tick_accumulator: f32,
    pub tick_rate: f32,
    pub paused: bool,
    pub single_step: bool,
}

impl FrameTiming {
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            last_dt: 0.0,
            tick_accumulator: 0.0,
            tick_rate: 10.0,
            paused: false,
            single_step: false,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.frame_count += 1;
        self.last_dt = dt;
    }

    /// Returns how many simulation ticks should run this frame.
    pub fn ticks_due(&mut self, dt: f32) -> u32 {
        if self.paused && !self.single_step {
            return 0;
        }

        if self.single_step {
            self.single_step = false;
            return 1;
        }

        let interval = 1.0 / self.tick_rate;
        self.tick_accumulator += dt;

        // Spiral of death prevention: if we've fallen behind by 3+ intervals, reset
        if self.tick_accumulator > interval * 3.0 {
            self.tick_accumulator = 0.0;
            return 3;
        }

        let mut ticks = 0u32;
        while self.tick_accumulator >= interval && ticks < 3 {
            self.tick_accumulator -= interval;
            ticks += 1;
        }

        ticks
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    pub fn request_single_step(&mut self) {
        self.single_step = true;
    }

    pub fn set_tick_rate(&mut self, rate: f32) {
        self.tick_rate = rate.clamp(1.0, 60.0);
    }
}
