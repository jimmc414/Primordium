import wasmInit, { init, frame, on_mouse_move, on_scroll, on_key_down, set_paused, single_step, set_tick_rate, set_tool, set_brush_radius, set_overlay_mode, on_mouse_down, request_pick, get_pick_result, get_stats, set_param } from '../crates/host/pkg/host.js';

async function main() {
    const errorDiv = document.getElementById('error-msg');

    try {
        await wasmInit();
        await init();
    } catch (e) {
        console.error('Init failed:', e);
        errorDiv.textContent = 'WebGPU initialization failed.\nPlease use Chrome 113+ with WebGPU enabled.\n\n' + e;
        errorDiv.style.display = 'block';
        return;
    }

    const canvas = document.getElementById('gpu-canvas');

    // Resize canvas to match device pixel ratio
    function resize() {
        const dpr = window.devicePixelRatio || 1;
        canvas.width = Math.floor(canvas.clientWidth * dpr);
        canvas.height = Math.floor(canvas.clientHeight * dpr);
    }
    resize();
    window.addEventListener('resize', resize);

    // Mouse input: right-drag = orbit, middle-drag = pan, left-click = tool
    canvas.addEventListener('mousemove', (e) => {
        on_mouse_move(e.movementX, e.movementY, e.buttons);
    });

    canvas.addEventListener('wheel', (e) => {
        e.preventDefault();
        on_scroll(e.deltaY);
    }, { passive: false });

    canvas.addEventListener('mousedown', (e) => {
        if (e.button === 0 && e.shiftKey) {
            // Shift+click: voxel inspector
            request_pick(e.offsetX, e.offsetY, canvas.clientWidth, canvas.clientHeight);
        } else if (e.button === 0) {
            // Left click: tool action
            on_mouse_down(e.offsetX, e.offsetY, canvas.clientWidth, canvas.clientHeight);
        }
    });

    // Keyboard input
    window.addEventListener('keydown', (e) => {
        on_key_down(e.key);
    });

    // Prevent context menu on right-click
    canvas.addEventListener('contextmenu', (e) => e.preventDefault());

    // Expose bridge functions for ui.js
    window._bridge = {
        set_tool,
        set_brush_radius,
        set_overlay_mode,
        set_paused,
        single_step,
        set_tick_rate,
        get_stats,
        get_pick_result,
        request_pick,
        set_param,
    };

    // Notify ui.js that bridge is ready
    window.dispatchEvent(new Event('bridge-ready'));

    // Animation loop with stats polling
    let lastTime = performance.now();
    let statsPollCounter = 0;
    function loop(now) {
        const dt = (now - lastTime) / 1000.0;
        lastTime = now;
        frame(dt);

        // Poll stats every ~5 frames
        statsPollCounter++;
        if (statsPollCounter >= 5) {
            statsPollCounter = 0;
            const stats = get_stats();
            if (stats && window._onStats) {
                window._onStats(stats);
            }
        }

        // Check for pick result
        const pick = get_pick_result();
        if (pick && window._onPick) {
            window._onPick(pick);
        }

        requestAnimationFrame(loop);
    }
    requestAnimationFrame(loop);
}

main();
