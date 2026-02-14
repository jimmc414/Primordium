use crate::SimEngine;

impl SimEngine {
    pub fn tick(&mut self, encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue) {
        // 1. Update tick_count in params and upload
        self.params.tick_count = self.tick_count as f32;
        self.params_uniform.upload(queue, &self.params);

        // 2. Clear intent buffer (SIM-2: prevent ghost intents)
        encoder.clear_buffer(self.buffers.intent_buffer(), 0, None);

        // 3. Select bind groups by parity (even=read A, odd=read B)
        let (intent_bg, resolve_bg) = if self.buffers.current_read_is_a() {
            (&self.intent_bg_even, &self.resolve_bg_even)
        } else {
            (&self.intent_bg_odd, &self.resolve_bg_odd)
        };

        let wg = self.buffers.grid_size() / 4;

        // 4. Dispatch 1: intent_declaration
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("intent_declaration_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.intent_declaration);
            pass.set_bind_group(0, intent_bg, &[]);
            pass.dispatch_workgroups(wg, wg, wg);
        }

        // 5. Dispatch 2: resolve_execute
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("resolve_execute_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.resolve_execute);
            pass.set_bind_group(0, resolve_bg, &[]);
            pass.dispatch_workgroups(wg, wg, wg);
        }

        // 6. Swap buffers + increment tick
        self.buffers.swap();
        self.tick_count += 1;
    }
}
