// ============================================================
// ui.js — M7: Full UI: tool palette, stats, graph, inspector,
// parameter sliders, overlay modes, tick rate controls.
// ============================================================

// ---- 8a. Tool palette (carried from M4) ----
const tools = [
    { id: 1, name: 'Wall', key: '1' },
    { id: 2, name: 'Energy', key: '2' },
    { id: 3, name: 'Nutrient', key: '3' },
    { id: 4, name: 'Seed', key: '4' },
    { id: 5, name: 'Toxin', key: '5' },
    { id: 6, name: 'Remove', key: '6' },
    { id: 7, name: 'Heat', key: '7' },
    { id: 8, name: 'Cold', key: '8' },
];

let activeTool = 0;

function createToolbar() {
    const toolbar = document.getElementById('toolbar');
    if (!toolbar) return;

    tools.forEach(tool => {
        const btn = document.createElement('button');
        btn.className = 'tool-btn';
        btn.textContent = `${tool.key} ${tool.name}`;
        btn.dataset.toolId = tool.id;
        btn.addEventListener('click', () => selectTool(tool.id));
        toolbar.appendChild(btn);
    });

    // Brush radius slider
    const sliderLabel = document.createElement('label');
    sliderLabel.className = 'brush-label';
    sliderLabel.textContent = 'Radius: 0';

    const slider = document.createElement('input');
    slider.type = 'range';
    slider.id = 'brush-radius';
    slider.min = '0';
    slider.max = '5';
    slider.value = '0';
    slider.addEventListener('input', () => {
        const val = parseInt(slider.value);
        sliderLabel.textContent = `Radius: ${val}`;
        if (window._bridge) window._bridge.set_brush_radius(val);
    });

    toolbar.appendChild(sliderLabel);
    toolbar.appendChild(slider);

    // ---- 8f. Overlay mode buttons ----
    const overlayDiv = document.createElement('div');
    overlayDiv.style.marginTop = '8px';
    const overlayModes = ['Normal', 'Temp', 'Energy', 'Pop'];
    let currentOverlay = 0;
    overlayModes.forEach((name, i) => {
        const btn = document.createElement('button');
        btn.className = 'overlay-btn' + (i === 0 ? ' active' : '');
        btn.textContent = name;
        btn.addEventListener('click', () => {
            currentOverlay = i;
            overlayDiv.querySelectorAll('.overlay-btn').forEach((b, j) => {
                b.classList.toggle('active', j === i);
            });
            if (window._bridge) window._bridge.set_overlay_mode(i);
        });
        overlayDiv.appendChild(btn);
    });
    toolbar.appendChild(overlayDiv);

    // ---- 8g. Tick rate controls ----
    const tickDiv = document.createElement('div');
    tickDiv.style.marginTop = '8px';

    const tickLabel = document.createElement('label');
    tickLabel.className = 'brush-label';
    tickLabel.textContent = 'Tick: 10/s';

    const tickSlider = document.createElement('input');
    tickSlider.type = 'range';
    tickSlider.min = '1';
    tickSlider.max = '60';
    tickSlider.value = '10';
    tickSlider.style.width = '100px';
    tickSlider.style.accentColor = '#4af';
    tickSlider.addEventListener('input', () => {
        const val = parseInt(tickSlider.value);
        tickLabel.textContent = `Tick: ${val}/s`;
        if (window._bridge) window._bridge.set_tick_rate(val);
    });

    const pauseBtn = document.createElement('button');
    pauseBtn.className = 'tool-btn';
    pauseBtn.textContent = 'P Pause';
    let paused = false;
    pauseBtn.addEventListener('click', () => {
        paused = !paused;
        pauseBtn.textContent = paused ? 'P Resume' : 'P Pause';
        pauseBtn.classList.toggle('active', paused);
        if (window._bridge) window._bridge.set_paused(paused);
    });

    const stepBtn = document.createElement('button');
    stepBtn.className = 'tool-btn';
    stepBtn.textContent = 'N Step';
    stepBtn.addEventListener('click', () => {
        if (window._bridge) window._bridge.single_step();
    });

    tickDiv.appendChild(tickLabel);
    tickDiv.appendChild(tickSlider);
    tickDiv.appendChild(pauseBtn);
    tickDiv.appendChild(stepBtn);
    toolbar.appendChild(tickDiv);

    // ---- 8h. Preset buttons ----
    const presetDiv = document.createElement('div');
    presetDiv.style.marginTop = '8px';
    const presetLabel = document.createElement('label');
    presetLabel.className = 'brush-label';
    presetLabel.textContent = 'Presets';
    presetDiv.appendChild(presetLabel);

    const presets = [
        { id: 0, name: 'Petri Dish' },
        { id: 1, name: 'Gradient' },
        { id: 2, name: 'Arena' },
    ];
    presets.forEach(p => {
        const btn = document.createElement('button');
        btn.className = 'preset-btn';
        btn.textContent = p.name;
        btn.addEventListener('click', () => {
            if (window._bridge) window._bridge.load_preset(p.id);
        });
        presetDiv.appendChild(btn);
    });
    toolbar.appendChild(presetDiv);
}

function selectTool(id) {
    activeTool = (activeTool === id) ? 0 : id;
    if (window._bridge) window._bridge.set_tool(activeTool);
    updateButtons();
}

function updateButtons() {
    document.querySelectorAll('#toolbar > .tool-btn').forEach(btn => {
        const toolId = parseInt(btn.dataset.toolId);
        if (!isNaN(toolId)) {
            btn.classList.toggle('active', toolId === activeTool);
        }
    });
}

// ---- 8b. Stats display ----
let lastFps = 0;
let frameCount = 0;
let fpsTimer = 0;

function updateStats(stats) {
    const panel = document.getElementById('stats-panel');
    if (!panel) return;

    // FPS tracking
    frameCount++;
    const now = performance.now();
    if (now - fpsTimer > 1000) {
        lastFps = frameCount;
        frameCount = 0;
        fpsTimer = now;
    }

    const avgEnergy = stats.population > 0
        ? Math.round(stats.total_energy / stats.population)
        : 0;

    const gs = window._gridSize || '?';
    panel.innerHTML =
        `<span class="stat-label">Grid</span><span class="stat-value">${gs}³</span><br>` +
        `<span class="stat-label">Population</span><span class="stat-value">${stats.population}</span><br>` +
        `<span class="stat-label">Species</span><span class="stat-value">${stats.species_count}</span><br>` +
        `<span class="stat-label">Avg Energy</span><span class="stat-value">${avgEnergy}</span><br>` +
        `<span class="stat-label">Max Energy</span><span class="stat-value">${stats.max_energy}</span><br>` +
        `<span class="stat-label">FPS</span><span class="stat-value">${lastFps}</span>`;
}

// ---- 8c. Population graph ----
const graphHistory = []; // array of { tick, species: Map<id, count> }
const MAX_HISTORY = 500;
const speciesColors = new Map();

function getSpeciesColor(speciesId) {
    if (speciesColors.has(speciesId)) return speciesColors.get(speciesId);
    const hue = ((speciesId * 0.618033988749) % 1.0) * 360;
    const color = `hsl(${hue}, 80%, 60%)`;
    speciesColors.set(speciesId, color);
    return color;
}

function updateGraph(stats) {
    const canvas = document.getElementById('population-graph');
    if (!canvas) return;
    const ctx = canvas.getContext('2d');

    // Record data point
    const speciesMap = new Map();
    if (stats.species) {
        for (const [sid, count] of stats.species) {
            speciesMap.set(sid, count);
        }
    }
    graphHistory.push({ species: speciesMap });
    if (graphHistory.length > MAX_HISTORY) graphHistory.shift();

    // Find top 5 species across all history
    const totalCounts = new Map();
    for (const entry of graphHistory) {
        for (const [sid, count] of entry.species) {
            totalCounts.set(sid, (totalCounts.get(sid) || 0) + count);
        }
    }
    const topSpecies = [...totalCounts.entries()]
        .sort((a, b) => b[1] - a[1])
        .slice(0, 5)
        .map(e => e[0]);

    // Find max population for Y axis
    let maxPop = 10;
    for (const entry of graphHistory) {
        for (const [, count] of entry.species) {
            maxPop = Math.max(maxPop, count);
        }
    }

    // Draw
    const w = canvas.width;
    const h = canvas.height;
    ctx.clearRect(0, 0, w, h);

    // Background
    ctx.fillStyle = 'rgba(15, 15, 15, 0.85)';
    ctx.fillRect(0, 0, w, h);

    // Grid lines
    ctx.strokeStyle = '#222';
    ctx.lineWidth = 1;
    for (let i = 1; i < 4; i++) {
        const y = (h * i) / 4;
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(w, y);
        ctx.stroke();
    }

    // Draw lines per species
    const len = graphHistory.length;
    if (len < 2) return;

    for (const sid of topSpecies) {
        ctx.strokeStyle = getSpeciesColor(sid);
        ctx.lineWidth = 1.5;
        ctx.beginPath();
        let started = false;
        for (let i = 0; i < len; i++) {
            const count = graphHistory[i].species.get(sid) || 0;
            const x = (i / (MAX_HISTORY - 1)) * w;
            const y = h - (count / maxPop) * (h - 4) - 2;
            if (!started) {
                ctx.moveTo(x, y);
                started = true;
            } else {
                ctx.lineTo(x, y);
            }
        }
        ctx.stroke();
    }

    // Y-axis label
    ctx.fillStyle = '#555';
    ctx.font = '10px monospace';
    ctx.fillText(String(maxPop), 4, 12);
    ctx.fillText('0', 4, h - 4);
}

// ---- 8d. Voxel inspector tooltip ----
const GENOME_NAMES = [
    'metabolic_eff', 'metabolic_rate', 'repl_threshold', 'mutation_rate',
    'movement_bias', 'chemotaxis', 'toxin_resist', 'predation_cap',
    'pred_aggression', 'photosyn_rate', 'energy_split', 'reserved_11',
    'reserved_12', 'reserved_13', 'reserved_14', 'reserved_15',
];

const VOXEL_TYPE_NAMES = ['Empty', 'Wall', 'Nutrient', 'EnergySource', 'Protocell', 'Waste', 'HeatSource', 'ColdSource'];

let lastPickX = 0, lastPickY = 0;

function showInspector(pick, screenX, screenY) {
    const tip = document.getElementById('inspector-tooltip');
    if (!tip) return;

    const typeName = VOXEL_TYPE_NAMES[pick.voxel_type] || 'Unknown';
    let html = `<div class="pick-header">${typeName} (${pick.x}, ${pick.y}, ${pick.z})</div>`;
    html += `Energy: ${pick.energy}<br>`;
    html += `Age: ${pick.age}<br>`;

    if (pick.voxel_type === 4) {
        html += `Species: ${pick.species_id}<br>`;
        html += '<br>';
        const genome = pick.genome;
        for (let i = 0; i < 16; i++) {
            html += `<span class="genome-row">${GENOME_NAMES[i]}: ${genome[i]}</span><br>`;
        }
    }

    tip.innerHTML = html;
    tip.style.display = 'block';
    tip.style.left = Math.min(screenX + 16, window.innerWidth - 340) + 'px';
    tip.style.top = Math.min(screenY + 16, window.innerHeight - 300) + 'px';
}

function hideInspector() {
    const tip = document.getElementById('inspector-tooltip');
    if (tip) tip.style.display = 'none';
}

// ---- 8e. Parameter sliders ----
const PARAMS = [
    { name: 'dt', min: 0.01, max: 1.0, step: 0.01, default: 0.016 },
    { name: 'nutrient_spawn_rate', min: 0, max: 0.01, step: 0.0001, default: 0.001 },
    { name: 'waste_decay_ticks', min: 10, max: 500, step: 1, default: 100 },
    { name: 'nutrient_recycle_rate', min: 0, max: 1, step: 0.01, default: 0.5 },
    { name: 'movement_energy_cost', min: 0, max: 20, step: 0.5, default: 5 },
    { name: 'base_ambient_temp', min: 0, max: 1, step: 0.01, default: 0.5 },
    { name: 'metabolic_cost_base', min: 0, max: 5, step: 0.1, default: 2 },
    { name: 'replication_energy_min', min: 50, max: 1000, step: 10, default: 200 },
    { name: 'energy_from_nutrient', min: 10, max: 500, step: 5, default: 50 },
    { name: 'energy_from_source', min: 10, max: 500, step: 5, default: 10 },
    { name: 'diffusion_rate', min: 0, max: 0.25, step: 0.005, default: 0.1 },
    { name: 'temp_sensitivity', min: 0, max: 2, step: 0.05, default: 1 },
    { name: 'predation_energy_fraction', min: 0, max: 1, step: 0.05, default: 0.5 },
    { name: 'max_energy', min: 100, max: 65535, step: 100, default: 1000 },
];

function createParamsPanel() {
    const panel = document.getElementById('params-panel');
    if (!panel) return;

    const toggle = document.createElement('div');
    toggle.className = 'params-toggle';
    toggle.textContent = 'Parameters [click to toggle]';
    toggle.addEventListener('click', () => {
        panel.classList.toggle('collapsed');
    });
    panel.appendChild(toggle);
    panel.classList.add('collapsed');

    for (const p of PARAMS) {
        const row = document.createElement('div');
        row.className = 'param-row';

        const label = document.createElement('label');
        const valSpan = document.createElement('span');
        valSpan.className = 'param-val';
        valSpan.textContent = String(p.default);

        label.textContent = p.name;
        label.appendChild(valSpan);

        const input = document.createElement('input');
        input.type = 'range';
        input.min = String(p.min);
        input.max = String(p.max);
        input.step = String(p.step);
        input.value = String(p.default);
        input.addEventListener('input', () => {
            const val = parseFloat(input.value);
            valSpan.textContent = val.toFixed(p.step < 0.01 ? 4 : p.step < 1 ? 2 : 0);
            if (window._bridge) window._bridge.set_param(p.name, val);
        });

        row.appendChild(label);
        row.appendChild(input);
        panel.appendChild(row);
    }
}

// ---- Global event wiring ----

// Stats callback (called by input.js animation loop)
window._onStats = (stats) => {
    updateStats(stats);
    updateGraph(stats);
};

// Pick callback (called by input.js animation loop)
window._onPick = (pick) => {
    showInspector(pick, lastPickX, lastPickY);
};

// Track mouse position for inspector tooltip placement
window.addEventListener('mousedown', (e) => {
    if (e.shiftKey) {
        lastPickX = e.clientX;
        lastPickY = e.clientY;
    } else {
        hideInspector();
    }
});

window.addEventListener('keydown', (e) => {
    const keyMap = { '1': 1, '2': 2, '3': 3, '4': 4, '5': 5, '6': 6, '7': 7, '8': 8 };
    if (e.key === '?') {
        const overlay = document.getElementById('shortcut-overlay');
        if (overlay) overlay.classList.toggle('visible');
    } else if (keyMap[e.key] !== undefined) {
        activeTool = keyMap[e.key];
        updateButtons();
    } else if (e.key === 'Escape') {
        // Close shortcut overlay if open
        const overlay = document.getElementById('shortcut-overlay');
        if (overlay && overlay.classList.contains('visible')) {
            overlay.classList.remove('visible');
            return;
        }
        activeTool = 0;
        updateButtons();
        hideInspector();
    }
});

// ---- Init ----
function initUI() {
    createToolbar();
    createParamsPanel();
}

if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initUI);
} else {
    initUI();
}
