use wasm_bindgen::prelude::*;
use std::cell::RefCell;
use glam::Vec4;
use js_sys;

use crate::App;

thread_local! {
    pub static APP: RefCell<Option<App>> = RefCell::new(None);
}

#[wasm_bindgen]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    None = 0,
    Wall = 1,
    EnergySource = 2,
    Nutrient = 3,
    Seed = 4,
    Toxin = 5,
    Remove = 6,
    HeatSource = 7,
    ColdSource = 8,
}

#[wasm_bindgen]
pub fn on_mouse_move(dx: f32, dy: f32, buttons: u32) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            if buttons & 2 != 0 {
                // Right mouse button: orbit
                app.camera.orbit(dx, dy);
            } else if buttons & 4 != 0 {
                // Middle mouse button: pan
                app.camera.pan(dx, dy);
            }
        }
    });
}

#[wasm_bindgen]
pub fn on_scroll(delta: f32) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            app.camera.zoom(delta);
        }
    });
}

#[wasm_bindgen]
pub fn on_key_down(key: String) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            match key.as_str() {
                "c" | "C" => app.camera.cycle_clip_axis(),
                "ArrowUp" => app.camera.adjust_clip_position(0.02),
                "ArrowDown" => app.camera.adjust_clip_position(-0.02),
                "p" | "P" => app.timing.toggle_pause(),
                "n" | "N" => app.timing.request_single_step(),
                "1" => app.current_tool = Tool::Wall,
                "2" => app.current_tool = Tool::EnergySource,
                "3" => app.current_tool = Tool::Nutrient,
                "4" => app.current_tool = Tool::Seed,
                "5" => app.current_tool = Tool::Toxin,
                "6" => app.current_tool = Tool::Remove,
                "7" => app.current_tool = Tool::HeatSource,
                "8" => app.current_tool = Tool::ColdSource,
                "t" | "T" => app.overlay_mode = (app.overlay_mode + 1) % 4,
                "Escape" => app.current_tool = Tool::None,
                _ => {}
            }
        }
    });
}

#[wasm_bindgen]
pub fn set_paused(paused: bool) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            app.timing.set_paused(paused);
        }
    });
}

#[wasm_bindgen]
pub fn single_step() {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            app.timing.request_single_step();
        }
    });
}

#[wasm_bindgen]
pub fn set_tick_rate(rate: f32) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            app.timing.set_tick_rate(rate);
        }
    });
}

#[wasm_bindgen]
pub fn set_tool(tool_id: u32) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            app.current_tool = match tool_id {
                0 => Tool::None,
                1 => Tool::Wall,
                2 => Tool::EnergySource,
                3 => Tool::Nutrient,
                4 => Tool::Seed,
                5 => Tool::Toxin,
                6 => Tool::Remove,
                7 => Tool::HeatSource,
                8 => Tool::ColdSource,
                _ => Tool::None,
            };
        }
    });
}

#[wasm_bindgen]
pub fn set_overlay_mode(mode: u32) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            app.overlay_mode = mode;
        }
    });
}

#[wasm_bindgen]
pub fn set_brush_radius(radius: u32) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            app.brush_radius = radius.min(5);
        }
    });
}

#[wasm_bindgen]
pub fn request_pick(canvas_x: f32, canvas_y: f32, canvas_w: f32, canvas_h: f32) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            let nx = canvas_x / canvas_w;
            let ny = canvas_y / canvas_h;
            let gs = app.sim_engine.grid_size();
            if let Some((x, y, z)) = ray_cast_grid(&app.camera, nx, ny, gs) {
                app.pick_coords = Some((x, y, z));
                app.pick_requested = true;
                app.latest_pick = None;
            }
        }
    });
}

#[wasm_bindgen]
pub fn get_pick_result() -> JsValue {
    APP.with(|app| {
        let borrow = app.borrow();
        if let Some(ref app) = *borrow {
            if let Some(ref pick) = app.latest_pick {
                let obj = js_sys::Object::new();
                let _ = js_sys::Reflect::set(&obj, &"x".into(), &JsValue::from(pick.x));
                let _ = js_sys::Reflect::set(&obj, &"y".into(), &JsValue::from(pick.y));
                let _ = js_sys::Reflect::set(&obj, &"z".into(), &JsValue::from(pick.z));
                let _ = js_sys::Reflect::set(&obj, &"voxel_type".into(), &JsValue::from(pick.voxel_type));
                let _ = js_sys::Reflect::set(&obj, &"energy".into(), &JsValue::from(pick.energy));
                let _ = js_sys::Reflect::set(&obj, &"age".into(), &JsValue::from(pick.age));
                let _ = js_sys::Reflect::set(&obj, &"species_id".into(), &JsValue::from(pick.species_id));
                let genome = js_sys::Array::new();
                for b in &pick.genome {
                    genome.push(&JsValue::from(*b));
                }
                let _ = js_sys::Reflect::set(&obj, &"genome".into(), &genome);
                return obj.into();
            }
        }
        JsValue::NULL
    })
}

#[wasm_bindgen]
pub fn get_stats() -> JsValue {
    APP.with(|app| {
        let borrow = app.borrow();
        if let Some(ref app) = *borrow {
            if let Some(ref stats) = app.latest_stats {
                let obj = js_sys::Object::new();
                let _ = js_sys::Reflect::set(&obj, &"population".into(), &JsValue::from(stats.population));
                let _ = js_sys::Reflect::set(&obj, &"total_energy".into(), &JsValue::from(stats.total_energy));
                let _ = js_sys::Reflect::set(&obj, &"species_count".into(), &JsValue::from(stats.species_count));
                let _ = js_sys::Reflect::set(&obj, &"max_energy".into(), &JsValue::from(stats.max_energy));
                let species = js_sys::Array::new();
                for (sid, count) in &stats.species_histogram {
                    let entry = js_sys::Array::new();
                    entry.push(&JsValue::from(*sid));
                    entry.push(&JsValue::from(*count));
                    species.push(&entry);
                }
                let _ = js_sys::Reflect::set(&obj, &"species".into(), &species);
                return obj.into();
            }
        }
        JsValue::NULL
    })
}

#[wasm_bindgen]
pub fn load_preset(preset_id: u32) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            app.sim_engine.reset_tick_count();
            app.sim_engine.initialize_grid_with_preset(&app.gpu.queue, preset_id);
            app.latest_stats = None;
            app.stats_tick_counter = 0;
            app.stats_state = crate::ReadbackState::Idle;
        }
    });
}

#[wasm_bindgen]
pub fn run_benchmark() -> u32 {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            let count = app.sim_engine.seed_benchmark(&app.gpu.queue);
            app.latest_stats = None;
            app.stats_tick_counter = 0;
            app.stats_state = crate::ReadbackState::Idle;
            count
        } else {
            0
        }
    })
}

#[wasm_bindgen]
pub fn get_grid_size() -> u32 {
    APP.with(|app| {
        let borrow = app.borrow();
        if let Some(ref app) = *borrow {
            app.sim_engine.grid_size()
        } else {
            0
        }
    })
}

#[wasm_bindgen]
pub fn set_param(name: &str, value: f32) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            match name {
                "dt" => app.sim_engine.params.dt = value,
                "nutrient_spawn_rate" => app.sim_engine.params.nutrient_spawn_rate = value,
                "waste_decay_ticks" => app.sim_engine.params.waste_decay_ticks = value,
                "nutrient_recycle_rate" => app.sim_engine.params.nutrient_recycle_rate = value,
                "movement_energy_cost" => app.sim_engine.params.movement_energy_cost = value,
                "base_ambient_temp" => app.sim_engine.params.base_ambient_temp = value,
                "metabolic_cost_base" => app.sim_engine.params.metabolic_cost_base = value,
                "replication_energy_min" => app.sim_engine.params.replication_energy_min = value,
                "energy_from_nutrient" => app.sim_engine.params.energy_from_nutrient = value,
                "energy_from_source" => app.sim_engine.params.energy_from_source = value,
                "diffusion_rate" => app.sim_engine.params.diffusion_rate = value,
                "temp_sensitivity" => app.sim_engine.params.temp_sensitivity = value,
                "predation_energy_fraction" => app.sim_engine.params.predation_energy_fraction = value,
                "max_energy" => app.sim_engine.params.max_energy = value,
                _ => {}
            }
        }
    });
}

#[wasm_bindgen]
pub fn on_mouse_down(canvas_x: f32, canvas_y: f32, canvas_w: f32, canvas_h: f32) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            if app.current_tool == Tool::None {
                return;
            }

            let nx = canvas_x / canvas_w;
            let ny = canvas_y / canvas_h;
            let gs = app.sim_engine.grid_size();

            if let Some((x, y, z)) = ray_cast_grid(&app.camera, nx, ny, gs) {
                let cmd = match app.current_tool {
                    Tool::Wall => types::Command::new(
                        types::CommandType::PlaceVoxel, x, y, z, app.brush_radius, 1, 0,
                    ),
                    Tool::EnergySource => types::Command::new(
                        types::CommandType::PlaceVoxel, x, y, z, app.brush_radius, 3, 0,
                    ),
                    Tool::Nutrient => types::Command::new(
                        types::CommandType::PlaceVoxel, x, y, z, app.brush_radius, 2, 0,
                    ),
                    Tool::Seed => types::Command::new(
                        types::CommandType::SeedProtocells, x, y, z, app.brush_radius, 500, 0,
                    ),
                    Tool::Toxin => types::Command::new(
                        types::CommandType::ApplyToxin, x, y, z, app.brush_radius, 128, 0,
                    ),
                    Tool::Remove => types::Command::new(
                        types::CommandType::RemoveVoxel, x, y, z, app.brush_radius, 0, 0,
                    ),
                    Tool::HeatSource => types::Command::new(
                        types::CommandType::PlaceVoxel, x, y, z, app.brush_radius, 6, 0,
                    ),
                    Tool::ColdSource => types::Command::new(
                        types::CommandType::PlaceVoxel, x, y, z, app.brush_radius, 7, 0,
                    ),
                    Tool::None => return,
                };
                app.pending_commands.push(cmd);
            }
        }
    });
}

/// CPU ray cast: intersect screen point with grid AABB, return nearest grid cell.
fn ray_cast_grid(camera: &renderer::camera::Camera, nx: f32, ny: f32, grid_size: u32) -> Option<(u32, u32, u32)> {
    let inv_vp = camera.view_projection_inverse();
    let gs = grid_size as f32;

    // Unproject near and far plane points from NDC
    let ndc_near = Vec4::new(nx * 2.0 - 1.0, 1.0 - ny * 2.0, -1.0, 1.0);
    let ndc_far = Vec4::new(nx * 2.0 - 1.0, 1.0 - ny * 2.0, 1.0, 1.0);

    let w_near = inv_vp * ndc_near;
    if w_near.w.abs() < 1e-6 {
        return None;
    }
    let origin = w_near.truncate() / w_near.w;

    let w_far = inv_vp * ndc_far;
    if w_far.w.abs() < 1e-6 {
        return None;
    }
    let far_pt = w_far.truncate() / w_far.w;

    let dir = (far_pt - origin).normalize();

    // Ray-AABB slab intersection with [0, gs]^3
    let mut t_min = f32::NEG_INFINITY;
    let mut t_max = f32::INFINITY;

    for i in 0..3 {
        let o = match i { 0 => origin.x, 1 => origin.y, _ => origin.z };
        let d = match i { 0 => dir.x, 1 => dir.y, _ => dir.z };
        if d.abs() < 1e-8 {
            if o < 0.0 || o > gs {
                return None;
            }
        } else {
            let t1 = (0.0 - o) / d;
            let t2 = (gs - o) / d;
            let t_near = t1.min(t2);
            let t_far = t1.max(t2);
            t_min = t_min.max(t_near);
            t_max = t_max.min(t_far);
            if t_min > t_max {
                return None;
            }
        }
    }

    // Get entry point (use t_min if positive, else origin is inside)
    let t = if t_min > 0.0 { t_min } else { 0.0 };
    let hit = origin + dir * t;

    // Snap to nearest integer grid coords, clamp to [0, gs-1]
    let x = (hit.x.round() as i32).clamp(0, grid_size as i32 - 1) as u32;
    let y = (hit.y.round() as i32).clamp(0, grid_size as i32 - 1) as u32;
    let z = (hit.z.round() as i32).clamp(0, grid_size as i32 - 1) as u32;

    Some((x, y, z))
}
