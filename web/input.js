import wasmInit, { init, frame, on_mouse_move, on_scroll, on_key_down, set_paused, single_step, set_tick_rate, set_tool, set_brush_radius, on_mouse_down } from '../crates/host/pkg/host.js';

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
        if (e.button === 0) {
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
    window._bridge = { set_tool, set_brush_radius };

    // Animation loop
    let lastTime = performance.now();
    function loop(now) {
        const dt = (now - lastTime) / 1000.0;
        lastTime = now;
        frame(dt);
        requestAnimationFrame(loop);
    }
    requestAnimationFrame(loop);
}

main();
