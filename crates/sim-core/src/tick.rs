use crate::SimEngine;

impl SimEngine {
    pub fn tick(&mut self, encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue, commands: &[types::Command]) {
        // 1. Update tick_count in params and upload
        self.params.tick_count = self.tick_count as f32;
        self.params_uniform.upload(queue, &self.params);

        // 2. Upload and dispatch apply_commands (only if commands exist)
        let command_count = commands.len().min(64) as u32;
        if command_count > 0 {
            // Write command count at byte offset 0
            queue.write_buffer(
                self.buffers.command_buffer(),
                0,
                bytemuck::bytes_of(&command_count),
            );
            // Write command words starting at byte offset 16 (word 4)
            for (i, cmd) in commands.iter().take(64).enumerate() {
                let words = cmd.to_words();
                let byte_offset = 16 + (i as u64) * 64; // 16 words * 4 bytes = 64
                queue.write_buffer(
                    self.buffers.command_buffer(),
                    byte_offset,
                    bytemuck::cast_slice(&words),
                );
            }

            let apply_cmd_bg = if self.buffers.current_read_is_a() {
                &self.apply_cmd_bg_even
            } else {
                &self.apply_cmd_bg_odd
            };

            let wg = self.buffers.grid_size() / 4;
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("apply_commands_pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipelines.apply_commands);
                pass.set_bind_group(0, apply_cmd_bg, &[]);
                pass.dispatch_workgroups(wg, wg, wg);
            }

            // Clear command count for next tick
            queue.write_buffer(
                self.buffers.command_buffer(),
                0,
                bytemuck::bytes_of(&0u32),
            );
        }

        // 3. Clear intent buffer (SIM-2: prevent ghost intents)
        encoder.clear_buffer(self.buffers.intent_buffer(), 0, None);

        // 4. Select bind groups by parity (even=read A, odd=read B)
        let (intent_bg, resolve_bg) = if self.buffers.current_read_is_a() {
            (&self.intent_bg_even, &self.resolve_bg_even)
        } else {
            (&self.intent_bg_odd, &self.resolve_bg_odd)
        };

        let wg = self.buffers.grid_size() / 4;

        // 5. Dispatch 1: intent_declaration
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("intent_declaration_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.intent_declaration);
            pass.set_bind_group(0, intent_bg, &[]);
            pass.dispatch_workgroups(wg, wg, wg);
        }

        // 6. Dispatch 2: resolve_execute
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("resolve_execute_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.resolve_execute);
            pass.set_bind_group(0, resolve_bg, &[]);
            pass.dispatch_workgroups(wg, wg, wg);
        }

        // 7. Swap buffers + increment tick
        self.buffers.swap();
        self.tick_count += 1;
    }
}
