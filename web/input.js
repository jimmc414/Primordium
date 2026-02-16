import wasmInit, { init, frame, on_mouse_move, on_scroll, on_key_down, set_paused, single_step, set_tick_rate, set_tool, set_brush_radius, set_overlay_mode, on_mouse_down, request_pick, get_pick_result, get_stats, set_param, load_preset, run_benchmark, get_grid_size } from '../crates/host/pkg/host.js';

async function main() {
    const errorDiv = document.getElementById('error-msg');

    const loadingScreen = document.getElementById('loading-screen');

    try {
        await wasmInit();
        await init();
    } catch (e) {
        console.error('Init failed:', e);
        if (loadingScreen) loadingScreen.remove();
        errorDiv.textContent = 'WebGPU initialization failed.\nPlease use Chrome 113+ with WebGPU enabled.\n\n' + e;
        errorDiv.style.display = 'block';
        return;
    }

    // Hide loading screen
    if (loadingScreen) {
        loadingScreen.classList.add('hidden');
        setTimeout(() => loadingScreen.remove(), 500);
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

    // Store grid size for UI
    window._gridSize = get_grid_size();

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
        load_preset,
        run_benchmark,
        get_grid_size,
    };

    // Notify ui.js that bridge is ready
    window.dispatchEvent(new Event('bridge-ready'));

    // Performance timing
    let perfAccum = 0;
    let perfFrames = 0;
    let perfLastLog = performance.now();

    // Animation loop with stats polling
    let lastTime = performance.now();
    let statsPollCounter = 0;
    function loop(now) {
        const dt = (now - lastTime) / 1000.0;
        lastTime = now;

        const t0 = performance.now();
        frame(dt);
        const t1 = performance.now();

        // Accumulate frame timing
        perfAccum += (t1 - t0);
        perfFrames++;
        if (t1 - perfLastLog >= 5000) {
            const avgMs = perfAccum / perfFrames;
            const fps = 1000 / avgMs;
            console.log(`[perf] avg frame: ${avgMs.toFixed(2)}ms, ~${fps.toFixed(0)} FPS (${perfFrames} frames in 5s)`);
            perfAccum = 0;
            perfFrames = 0;
            perfLastLog = t1;
        }

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

    // Expose benchmark function
    window.benchmark = function() {
        console.log('[benchmark] Seeding 30% occupancy...');
        const count = run_benchmark();
        console.log(`[benchmark] Placed ${count} protocells`);

        console.log('[benchmark] Running 100 sim ticks...');
        const st0 = performance.now();
        for (let i = 0; i < 100; i++) {
            frame(1/60);
        }
        const st1 = performance.now();
        const simMs = st1 - st0;
        const ticksPerSec = (100 / simMs) * 1000;
        console.log(`[benchmark] 100 ticks in ${simMs.toFixed(0)}ms = ${ticksPerSec.toFixed(0)} ticks/sec`);

        console.log('[benchmark] Running 300 render frames...');
        const rt0 = performance.now();
        for (let i = 0; i < 300; i++) {
            frame(1/60);
        }
        const rt1 = performance.now();
        const renderMs = rt1 - rt0;
        const renderFps = (300 / renderMs) * 1000;
        console.log(`[benchmark] 300 frames in ${renderMs.toFixed(0)}ms = ${renderFps.toFixed(0)} FPS`);

        const simPass = ticksPerSec >= 10;
        const fpsPass = renderFps >= 30;
        console.log(`[benchmark] Sim: ${simPass ? 'PASS' : 'FAIL'} (>=10 ticks/s), Render: ${fpsPass ? 'PASS' : 'FAIL'} (>=30 FPS)`);
    };
}

main();
