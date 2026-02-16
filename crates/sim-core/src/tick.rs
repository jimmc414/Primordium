use crate::{SimEngine, SimMode, DenseMode, SparseMode};

impl SimEngine {
    pub fn tick(&mut self, encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue, commands: &[types::Command]) {
        // 1. Update tick_count in params and upload
        self.params.tick_count = self.tick_count as f32;
        self.params_uniform.upload(queue, &self.params);

        // Upload brick table before any dispatches (sparse only)
        if let SimMode::Sparse(s) = &mut self.mode {
            s.grid.upload_if_dirty(queue);
        }

        match &mut self.mode {
            SimMode::Dense(d) => tick_dense(encoder, queue, commands, d),
            SimMode::Sparse(s) => tick_sparse(encoder, queue, commands, s),
        }

        // Post-tick: border allocation for sparse (every ~10 ticks)
        if let SimMode::Sparse(s) = &mut self.mode {
            s.border_alloc_counter += 1;
            if s.border_alloc_counter >= 10 {
                s.grid.proactive_border_alloc();
                s.border_alloc_counter = 0;
            }
        }

        // Swap buffers (voxel + temp) + increment tick
        match &mut self.mode {
            SimMode::Dense(d) => d.buffers.swap(),
            SimMode::Sparse(s) => s.buffers.swap(),
        }
        self.tick_count += 1;
    }
}

fn tick_dense(encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue, commands: &[types::Command], d: &DenseMode) {
    let wg = d.buffers.grid_size() / 4;

    // 2. Apply player commands (only if commands exist)
    let command_count = commands.len().min(64) as u32;
    if command_count > 0 {
        queue.write_buffer(d.buffers.command_buffer(), 0, bytemuck::bytes_of(&command_count));
        for (i, cmd) in commands.iter().take(64).enumerate() {
            let words = cmd.to_words();
            let byte_offset = 16 + (i as u64) * 64;
            queue.write_buffer(d.buffers.command_buffer(), byte_offset, bytemuck::cast_slice(&words));
        }

        let apply_cmd_bg = if d.buffers.current_read_is_a() {
            &d.apply_cmd_bg_even
        } else {
            &d.apply_cmd_bg_odd
        };

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("apply_commands_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&d.pipelines.apply_commands);
            pass.set_bind_group(0, apply_cmd_bg, &[]);
            pass.dispatch_workgroups(wg, wg, wg);
        }

        queue.write_buffer(d.buffers.command_buffer(), 0, bytemuck::bytes_of(&0u32));
    }

    // 3. Temperature diffusion
    let (temp_bg, intent_bg, resolve_bg) = if d.buffers.current_read_is_a() {
        (&d.temp_diffusion_bg_even, &d.intent_bg_even, &d.resolve_bg_even)
    } else {
        (&d.temp_diffusion_bg_odd, &d.intent_bg_odd, &d.resolve_bg_odd)
    };

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("temperature_diffusion_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&d.pipelines.temperature_diffusion);
        pass.set_bind_group(0, temp_bg, &[]);
        pass.dispatch_workgroups(wg, wg, wg);
    }

    // 4. Clear intent buffer
    encoder.clear_buffer(d.buffers.intent_buffer(), 0, None);

    // 5. Intent declaration
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("intent_declaration_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&d.pipelines.intent_declaration);
        pass.set_bind_group(0, intent_bg, &[]);
        pass.dispatch_workgroups(wg, wg, wg);
    }

    // 6. Resolve and execute
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("resolve_execute_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&d.pipelines.resolve_execute);
        pass.set_bind_group(0, resolve_bg, &[]);
        pass.dispatch_workgroups(wg, wg, wg);
    }

    // 7. Stats reduction
    encoder.clear_buffer(d.buffers.stats_buffer(), 0, None);

    let stats_bg = if d.buffers.current_read_is_a() {
        &d.stats_bg_even
    } else {
        &d.stats_bg_odd
    };

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("stats_reduction_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&d.pipelines.stats_reduction);
        pass.set_bind_group(0, stats_bg, &[]);
        let total_voxels = (d.buffers.grid_size() as u32).pow(3);
        let workgroups = (total_voxels + 63) / 64;
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    encoder.copy_buffer_to_buffer(
        d.buffers.stats_buffer(), 0,
        d.buffers.stats_staging_buffer(), 0,
        128,
    );
}

fn tick_sparse(encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue, commands: &[types::Command], s: &SparseMode) {
    // Sparse dispatch: full 256³ grid, threads in unallocated bricks exit early
    let wg = s.buffers.grid_size() / 4; // 64 for 256³

    // 2. Apply player commands
    let command_count = commands.len().min(64) as u32;
    if command_count > 0 {
        queue.write_buffer(s.buffers.command_buffer(), 0, bytemuck::bytes_of(&command_count));
        for (i, cmd) in commands.iter().take(64).enumerate() {
            let words = cmd.to_words();
            let byte_offset = 16 + (i as u64) * 64;
            queue.write_buffer(s.buffers.command_buffer(), byte_offset, bytemuck::cast_slice(&words));
        }

        let apply_cmd_bg = if s.buffers.current_read_is_a() {
            &s.apply_cmd_bg_even
        } else {
            &s.apply_cmd_bg_odd
        };

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("sparse_apply_commands_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&s.pipelines.apply_commands);
            pass.set_bind_group(0, apply_cmd_bg, &[]);
            pass.dispatch_workgroups(wg, wg, wg);
        }

        queue.write_buffer(s.buffers.command_buffer(), 0, bytemuck::bytes_of(&0u32));
    }

    // 3. Temperature diffusion
    let (temp_bg, intent_bg, resolve_bg) = if s.buffers.current_read_is_a() {
        (&s.temp_diffusion_bg_even, &s.intent_bg_even, &s.resolve_bg_even)
    } else {
        (&s.temp_diffusion_bg_odd, &s.intent_bg_odd, &s.resolve_bg_odd)
    };

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("sparse_temperature_diffusion_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&s.pipelines.temperature_diffusion);
        pass.set_bind_group(0, temp_bg, &[]);
        pass.dispatch_workgroups(wg, wg, wg);
    }

    // 4. Clear intent pool
    encoder.clear_buffer(s.buffers.intent_pool(), 0, None);

    // 5. Intent declaration
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("sparse_intent_declaration_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&s.pipelines.intent_declaration);
        pass.set_bind_group(0, intent_bg, &[]);
        pass.dispatch_workgroups(wg, wg, wg);
    }

    // 6. Resolve and execute
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("sparse_resolve_execute_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&s.pipelines.resolve_execute);
        pass.set_bind_group(0, resolve_bg, &[]);
        pass.dispatch_workgroups(wg, wg, wg);
    }

    // 7. Stats reduction
    encoder.clear_buffer(s.buffers.stats_buffer(), 0, None);

    let stats_bg = if s.buffers.current_read_is_a() {
        &s.stats_bg_even
    } else {
        &s.stats_bg_odd
    };

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("sparse_stats_reduction_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&s.pipelines.stats_reduction);
        pass.set_bind_group(0, stats_bg, &[]);
        // For sparse, iterate over pool slots: max_bricks * 512
        let total_pool_voxels = s.buffers.max_bricks() * 512;
        let workgroups = (total_pool_voxels + 63) / 64;
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    encoder.copy_buffer_to_buffer(
        s.buffers.stats_buffer(), 0,
        s.buffers.stats_staging_buffer(), 0,
        128,
    );
}
