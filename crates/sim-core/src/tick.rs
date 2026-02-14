use crate::SimEngine;

impl SimEngine {
    pub fn tick(&mut self, encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue) {
        // 1. Update tick_count in params and upload
        self.params.tick_count = self.tick_count as f32;
        self.params_uniform.upload(queue, &self.params);

        // 2. Select bind group (even=read A/write B, odd=read B/write A)
        let bg = if self.buffers.current_read_is_a() {
            &self.bind_group_even
        } else {
            &self.bind_group_odd
        };

        // 3. Dispatch resolve_execute
        let wg = self.buffers.grid_size() / 4;
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("resolve_execute_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.resolve_execute);
            pass.set_bind_group(0, bg, &[]);
            pass.dispatch_workgroups(wg, wg, wg);
        }

        // 4. Swap buffers + increment tick
        self.buffers.swap();
        self.tick_count += 1;
    }
}
