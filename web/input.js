import wasmInit, { init, frame, on_mouse_move, on_scroll, on_key_down, set_paused, single_step, set_tick_rate } from '../crates/host/pkg/host.js';

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

    // Mouse input
    canvas.addEventListener('mousemove', (e) => {
        on_mouse_move(e.movementX, e.movementY, e.buttons);
    });

    canvas.addEventListener('wheel', (e) => {
        e.preventDefault();
        on_scroll(e.deltaY);
    }, { passive: false });

    // Keyboard input
    window.addEventListener('keydown', (e) => {
        on_key_down(e.key);
    });

    // Prevent context menu on right-click
    canvas.addEventListener('contextmenu', (e) => e.preventDefault());

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
