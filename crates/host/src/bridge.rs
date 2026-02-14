use wasm_bindgen::prelude::*;
use std::cell::RefCell;

use crate::App;

thread_local! {
    pub static APP: RefCell<Option<App>> = RefCell::new(None);
}

#[wasm_bindgen]
pub fn on_mouse_move(dx: f32, dy: f32, buttons: u32) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            if buttons & 1 != 0 {
                // Left mouse button: orbit
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
