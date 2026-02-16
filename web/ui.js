// ============================================================
// ui.js — Full UI with tutorial system & UX improvements.
// Tool palette, stats, graph, inspector, parameter sliders,
// overlay modes, tick rate controls, tooltips, grouped params,
// genome interpretation, stats enhancements, camera hint, tutorial.
// ============================================================

// ---- Tool palette ----
const tools = [
    { id: 1, name: 'Wall', key: '1', desc: 'Place barriers that block movement and temperature' },
    { id: 2, name: 'Energy', key: '2', desc: 'Place energy sources that feed nearby protocells' },
    { id: 3, name: 'Nutrient', key: '3', desc: 'Place nutrients that protocells consume for energy' },
    { id: 4, name: 'Seed', key: '4', desc: 'Spawn a new protocell with a random genome' },
    { id: 5, name: 'Toxin', key: '5', desc: 'Place waste that damages protocells on contact' },
    { id: 6, name: 'Remove', key: '6', desc: 'Erase any voxel back to empty space' },
    { id: 7, name: 'Heat', key: '7', desc: 'Place heat sources that raise local temperature' },
    { id: 8, name: 'Cold', key: '8', desc: 'Place cold sources that lower local temperature' },
];

const OVERLAY_DESCS = {
    'Normal': 'Standard material view',
    'Temp': 'Temperature field (blue=cold, red=hot)',
    'Energy': 'Protocell energy levels (dark=low, bright=high)',
    'Pop': 'Species coloring by population',
};

const PRESET_DESCS = {
    'Petri Dish': 'Central colony surrounded by nutrients',
    'Gradient': 'Temperature gradient with hot and cold zones',
    'Arena': 'Walled arena with energy sources at corners',
};

let activeTool = 0;

function createToolbar() {
    const toolbar = document.getElementById('toolbar');
    if (!toolbar) return;

    tools.forEach(tool => {
        const btn = document.createElement('button');
        btn.className = 'tool-btn';
        btn.textContent = `${tool.key} ${tool.name}`;
        btn.dataset.toolId = tool.id;
        btn.dataset.tooltip = tool.desc;
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

    // ---- Overlay mode buttons ----
    const overlayDiv = document.createElement('div');
    overlayDiv.id = 'overlay-group';
    overlayDiv.style.marginTop = '8px';
    const overlayModes = ['Normal', 'Temp', 'Energy', 'Pop'];
    let currentOverlay = 0;
    overlayModes.forEach((name, i) => {
        const btn = document.createElement('button');
        btn.className = 'overlay-btn' + (i === 0 ? ' active' : '');
        btn.textContent = name;
        btn.dataset.tooltip = OVERLAY_DESCS[name];
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

    // ---- Tick rate controls ----
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

    // ---- Preset buttons ----
    const presetDiv = document.createElement('div');
    presetDiv.id = 'preset-group';
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
        btn.dataset.tooltip = PRESET_DESCS[p.name];
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

// ---- Tooltip manager ----
function createTooltipManager() {
    const tip = document.createElement('div');
    tip.id = 'ui-tooltip';
    document.body.appendChild(tip);

    document.addEventListener('mouseenter', (e) => {
        const el = e.target.closest('[data-tooltip]');
        if (!el) return;
        tip.textContent = el.dataset.tooltip;
        tip.style.display = 'block';
        const rect = el.getBoundingClientRect();
        let left = rect.right + 8;
        if (left + 260 > window.innerWidth) left = rect.left - 268;
        if (left < 4) left = 4;
        tip.style.left = left + 'px';
        tip.style.top = rect.top + 'px';
    }, true);

    document.addEventListener('mouseleave', (e) => {
        if (e.target.closest('[data-tooltip]')) {
            tip.style.display = 'none';
        }
    }, true);
}

// ---- Stats display ----
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

    const gs = window._gridSize || 0;
    const volume = gs * gs * gs;
    const occupancy = volume > 0 ? (stats.population / volume * 100).toFixed(1) : '0.0';

    // Health indicator
    let healthTag = '';
    if (stats.population > 0) {
        if (avgEnergy < 50) healthTag = '<span class="stat-indicator critical">critical</span>';
        else if (avgEnergy < 150) healthTag = '<span class="stat-indicator low">low</span>';
        else if (avgEnergy > 500) healthTag = '<span class="stat-indicator thriving">thriving</span>';
    }

    // Diversity indicator
    let divTag = '';
    if (stats.species_count === 1 && stats.population > 10) {
        divTag = '<span class="stat-indicator monoculture">monoculture</span>';
    } else if (stats.species_count > 20) {
        divTag = '<span class="stat-indicator diverse">diverse</span>';
    }

    panel.innerHTML =
        `<span class="stat-label">Grid</span><span class="stat-value">${gs || '?'}³</span><br>` +
        `<span class="stat-label">Population</span><span class="stat-value">${stats.population}</span><br>` +
        `<span class="stat-label">Occupancy</span><span class="stat-value">${occupancy}%</span><br>` +
        `<span class="stat-label">Species</span><span class="stat-value">${stats.species_count}</span>${divTag}<br>` +
        `<span class="stat-label">Avg Energy</span><span class="stat-value">${avgEnergy}</span>${healthTag}<br>` +
        `<span class="stat-label">Max Energy</span><span class="stat-value">${stats.max_energy}</span><br>` +
        `<span class="stat-label">FPS</span><span class="stat-value">${lastFps}</span>`;
}

// ---- Population graph ----
const graphHistory = []; // array of { species: Map<id, count> }
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

// ---- Voxel inspector with genome interpretation ----
const VOXEL_TYPE_NAMES = ['Empty', 'Wall', 'Nutrient', 'EnergySource', 'Protocell', 'Waste', 'HeatSource', 'ColdSource'];

const GENOME_INFO = [
    { name: 'Metabolic Efficiency', fn: v => `${Math.round(v / 255 * 100)}% \u2014 ${v < 64 ? 'low' : v < 192 ? 'moderate' : 'high'}` },
    { name: 'Metabolic Rate', fn: v => `${Math.round(v / 255 * 100)}% \u2014 ${v < 64 ? 'slow' : v < 192 ? 'moderate' : 'fast'}` },
    { name: 'Replication Threshold', fn: v => `${Math.round(v / 255 * 100)}% \u2014 ${v < 64 ? 'eager' : v < 192 ? 'selective' : 'conservative'}` },
    { name: 'Mutation Rate', fn: v => `${Math.round(v / 255 * 100)}% \u2014 ${v < 32 ? 'stable' : v < 128 ? 'moderate' : 'volatile'}` },
    { name: 'Movement Bias', fn: v => `${Math.round(v / 255 * 100)}% \u2014 ${v < 64 ? 'sedentary' : v < 192 ? 'moderate' : 'nomadic'}` },
    { name: 'Chemotaxis', fn: v => `${Math.round(v / 255 * 100)}% \u2014 ${v < 64 ? 'blind' : v < 192 ? 'moderate' : 'strong'}` },
    { name: 'Toxin Resistance', fn: v => `${Math.round(v / 255 * 100)}% \u2014 ${v < 64 ? 'vulnerable' : v < 192 ? 'resistant' : 'immune'}` },
    { name: 'Predation Capability', fn: v => v === 0 ? 'herbivore' : `${Math.round(v / 255 * 100)}% \u2014 ${v < 64 ? 'scavenger' : v < 192 ? 'hunter' : 'apex predator'}` },
    { name: 'Predation Aggression', fn: v => `${Math.round(v / 255 * 100)}% \u2014 ${v < 64 ? 'passive' : v < 192 ? 'moderate' : 'aggressive'}` },
    { name: 'Photosynthetic Rate', fn: v => v === 0 ? 'none' : `${Math.round(v / 255 * 100)}% \u2014 ${v < 64 ? 'low' : v < 192 ? 'moderate' : 'high'}` },
    { name: 'Energy Split Ratio', fn: v => `${Math.round(v / 255 * 100)}% to offspring` },
];

let lastPickX = 0, lastPickY = 0;

function showInspector(pick, screenX, screenY) {
    const tip = document.getElementById('inspector-tooltip');
    if (!tip) return;

    const typeName = VOXEL_TYPE_NAMES[pick.voxel_type] || 'Unknown';
    let html = `<div class="pick-header">${typeName} (${pick.x}, ${pick.y}, ${pick.z})</div>`;
    html += `Energy: ${pick.energy}<br>`;
    html += `Age: ${pick.age}<br>`;

    if (pick.voxel_type === 4) {
        html += `Species: ${pick.species_id}<br><br>`;
        const genome = pick.genome;
        for (let i = 0; i < GENOME_INFO.length; i++) {
            const gi = GENOME_INFO[i];
            const v = genome[i];
            html += `<span class="genome-row">${gi.name}: ${v} <span class="genome-interp">(${gi.fn(v)})</span></span><br>`;
        }
        // Reserved bytes 11-15: show only if non-zero
        for (let i = 11; i < 16; i++) {
            if (genome[i] !== 0) {
                html += `<span class="genome-row">reserved_${i}: ${genome[i]}</span><br>`;
            }
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

// ---- Parameter sliders (grouped with descriptions) ----
const PARAMS = [
    { name: 'nutrient_spawn_rate', min: 0, max: 0.01, step: 0.0001, default: 0.001, group: 'Resources', desc: 'Rate new nutrients appear in empty voxels' },
    { name: 'waste_decay_ticks', min: 10, max: 500, step: 1, default: 100, group: 'Resources', desc: 'Ticks before waste decomposes' },
    { name: 'nutrient_recycle_rate', min: 0, max: 1, step: 0.01, default: 0.5, group: 'Resources', desc: 'Fraction of waste that becomes nutrients' },
    { name: 'energy_from_nutrient', min: 10, max: 500, step: 5, default: 50, group: 'Energy', desc: 'Energy gained by consuming a nutrient' },
    { name: 'energy_from_source', min: 10, max: 500, step: 5, default: 10, group: 'Energy', desc: 'Energy gained from energy source voxels' },
    { name: 'max_energy', min: 100, max: 65535, step: 100, default: 1000, group: 'Energy', desc: 'Maximum energy a protocell can store' },
    { name: 'metabolic_cost_base', min: 0, max: 5, step: 0.1, default: 2, group: 'Energy', desc: 'Energy consumed per tick to stay alive' },
    { name: 'movement_energy_cost', min: 0, max: 20, step: 0.5, default: 5, group: 'Energy', desc: 'Energy spent per movement action' },
    { name: 'replication_energy_min', min: 50, max: 1000, step: 10, default: 200, group: 'Energy', desc: 'Minimum energy needed to replicate' },
    { name: 'base_ambient_temp', min: 0, max: 1, step: 0.01, default: 0.5, group: 'Temperature', desc: 'Background temperature (0=cold, 1=hot)' },
    { name: 'diffusion_rate', min: 0, max: 0.25, step: 0.005, default: 0.1, group: 'Temperature', desc: 'How fast temperature spreads (max 0.25)' },
    { name: 'temp_sensitivity', min: 0, max: 2, step: 0.05, default: 1, group: 'Temperature', desc: 'How much temperature affects metabolism' },
    { name: 'predation_energy_fraction', min: 0, max: 1, step: 0.05, default: 0.5, group: 'Combat', desc: 'Fraction of prey energy gained by predator' },
    { name: 'dt', min: 0.01, max: 1.0, step: 0.01, default: 0.016, group: 'Simulation', desc: 'Time step per tick (lower = more precise)' },
];

const PARAM_GROUP_ORDER = ['Resources', 'Energy', 'Temperature', 'Combat', 'Simulation'];

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

    // Group params by category
    const grouped = {};
    for (const p of PARAMS) {
        if (!grouped[p.group]) grouped[p.group] = [];
        grouped[p.group].push(p);
    }

    for (const groupName of PARAM_GROUP_ORDER) {
        const params = grouped[groupName];
        if (!params) continue;

        const header = document.createElement('div');
        header.className = 'param-group-header';
        header.textContent = groupName;
        panel.appendChild(header);

        for (const p of params) {
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

            const desc = document.createElement('div');
            desc.className = 'param-desc';
            desc.textContent = p.desc;

            row.appendChild(label);
            row.appendChild(input);
            row.appendChild(desc);
            panel.appendChild(row);
        }
    }
}

// ---- Camera hint ----
function createCameraHint() {
    const hint = document.getElementById('camera-hint');
    if (!hint) return;

    if (localStorage.getItem('primordium_tutorial_seen')) {
        hint.style.display = 'none';
        return;
    }

    // Fade out after 6 seconds
    setTimeout(() => hint.classList.add('hidden'), 6000);

    // Hide on first right-click
    const hideOnRight = (e) => {
        if (e.button === 2) {
            hint.classList.add('hidden');
            window.removeEventListener('mousedown', hideOnRight);
        }
    };
    window.addEventListener('mousedown', hideOnRight);
}

// ---- Tutorial engine ----
const TUTORIAL_STEPS = [
    {
        target: null,
        title: 'Welcome to Primordium',
        content: 'A voxel ecosystem simulator where protocells compete for resources, evolve, and form emergent species. Watch life unfold in 3D!',
    },
    {
        target: '#gpu-canvas',
        title: 'Camera Controls',
        content: 'Right-drag to orbit the camera around the grid. Middle-drag to pan. Scroll to zoom in and out. Left-click uses the selected tool.',
    },
    {
        target: '#preset-group',
        title: 'Presets',
        content: 'Quick-start scenarios. Petri Dish: central colony with nutrients. Gradient: temperature zones from hot to cold. Arena: walled enclosure with energy sources.',
    },
    {
        target: '#toolbar',
        title: 'Tools',
        content: 'Place voxels with left-click. Wall blocks movement. Energy and Nutrient feed protocells. Seed spawns a random protocell. Toxin places waste. Remove erases. Heat and Cold adjust temperature.',
    },
    {
        target: '#overlay-group',
        title: 'Overlay Modes',
        content: 'Change how the grid is visualized. Normal: standard view. Temp: temperature field (blue=cold, red=hot). Energy: protocell energy levels. Pop: species coloring.',
    },
    {
        target: '#stats-panel',
        title: 'Statistics',
        content: 'Population: living protocells. Species: distinct genome families. Avg Energy: colony health \u2014 above 150 is healthy, below 50 is critical. Occupancy: percentage of grid filled.',
    },
    {
        target: '#params-panel',
        title: 'Parameters',
        content: 'Fine-tune the simulation. Try increasing nutrient_spawn_rate for faster growth, or raise predation_energy_fraction for more predators. Click the header to expand.',
    },
    {
        target: null,
        title: 'Inspector & Tips',
        content: 'Shift+click any voxel to inspect its genome and stats. Press ? for keyboard shortcuts. Press Esc to close overlays. Enjoy experimenting!',
    },
];

let tutorialStep = -1;

function startTutorial() {
    tutorialStep = 0;
    const overlay = document.getElementById('tutorial-overlay');
    if (overlay) {
        overlay.style.display = 'block';
        showTutorialStep();
    }
}

function endTutorial() {
    tutorialStep = -1;
    const overlay = document.getElementById('tutorial-overlay');
    if (overlay) overlay.style.display = 'none';
    localStorage.setItem('primordium_tutorial_seen', '1');
}

function showTutorialStep() {
    if (tutorialStep < 0 || tutorialStep >= TUTORIAL_STEPS.length) return;
    const step = TUTORIAL_STEPS[tutorialStep];
    const spotlight = document.getElementById('tutorial-spotlight');
    const card = document.getElementById('tutorial-card');
    if (!card || !spotlight) return;

    // Update card content
    card.querySelector('.tutorial-title').textContent = step.title;
    card.querySelector('.tutorial-body').textContent = step.content;
    card.querySelector('.tutorial-step-indicator').textContent = `${tutorialStep + 1} / ${TUTORIAL_STEPS.length}`;

    // Back button visibility
    document.getElementById('tutorial-back').style.display = tutorialStep === 0 ? 'none' : '';

    // Next button text
    const nextBtn = document.getElementById('tutorial-next');
    nextBtn.textContent = tutorialStep === TUTORIAL_STEPS.length - 1 ? 'Finish' : 'Next';

    // Position spotlight and card
    if (step.target) {
        const el = document.querySelector(step.target);
        if (el) {
            const rect = el.getBoundingClientRect();
            const pad = 8;
            spotlight.style.display = 'block';
            spotlight.style.left = (rect.left - pad) + 'px';
            spotlight.style.top = (rect.top - pad) + 'px';
            spotlight.style.width = (rect.width + pad * 2) + 'px';
            spotlight.style.height = (rect.height + pad * 2) + 'px';
            positionTutorialCard(rect);
        } else {
            spotlight.style.display = 'none';
            centerTutorialCard();
        }
    } else {
        spotlight.style.display = 'none';
        centerTutorialCard();
    }
}

function centerTutorialCard() {
    const card = document.getElementById('tutorial-card');
    if (!card) return;
    card.style.left = '50%';
    card.style.top = '50%';
    card.style.transform = 'translate(-50%, -50%)';
}

function positionTutorialCard(targetRect) {
    const card = document.getElementById('tutorial-card');
    if (!card) return;
    card.style.transform = '';
    const margin = 16;
    const cardW = 340;

    // Try right of target
    let left = targetRect.right + margin;
    let top = targetRect.top;

    // If overflows right, try left side
    if (left + cardW > window.innerWidth - margin) {
        left = targetRect.left - cardW - margin;
    }

    // If still overflows, center below target
    if (left < margin) {
        left = Math.max(margin, (window.innerWidth - cardW) / 2);
        top = targetRect.bottom + margin;
    }

    // Clamp top to stay on screen
    top = Math.max(margin, Math.min(top, window.innerHeight - 280));

    card.style.left = left + 'px';
    card.style.top = top + 'px';
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
        // Close tutorial if active
        if (tutorialStep >= 0) {
            endTutorial();
            return;
        }
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

// Reposition tutorial card on resize
window.addEventListener('resize', () => {
    if (tutorialStep >= 0) showTutorialStep();
});

// ---- Init ----
function initUI() {
    createToolbar();
    createParamsPanel();
    createTooltipManager();
    createCameraHint();

    // Tutorial button wiring
    const nextBtn = document.getElementById('tutorial-next');
    const backBtn = document.getElementById('tutorial-back');
    const skipBtn = document.getElementById('tutorial-skip');

    if (nextBtn) nextBtn.addEventListener('click', () => {
        if (tutorialStep >= TUTORIAL_STEPS.length - 1) endTutorial();
        else { tutorialStep++; showTutorialStep(); }
    });
    if (backBtn) backBtn.addEventListener('click', () => {
        if (tutorialStep > 0) { tutorialStep--; showTutorialStep(); }
    });
    if (skipBtn) skipBtn.addEventListener('click', () => endTutorial());

    // Replay tutorial from help overlay
    const replayBtn = document.getElementById('replay-tutorial');
    if (replayBtn) {
        replayBtn.addEventListener('click', () => {
            document.getElementById('shortcut-overlay').classList.remove('visible');
            startTutorial();
        });
    }

    // Auto-start tutorial on first visit
    if (!localStorage.getItem('primordium_tutorial_seen')) {
        startTutorial();
    }
}

if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initUI);
} else {
    initUI();
}
