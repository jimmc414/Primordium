// Tool palette UI for M4
const tools = [
    { id: 1, name: 'Wall', key: '1' },
    { id: 2, name: 'Energy', key: '2' },
    { id: 3, name: 'Nutrient', key: '3' },
    { id: 4, name: 'Seed', key: '4' },
    { id: 5, name: 'Toxin', key: '5' },
    { id: 6, name: 'Remove', key: '6' },
];

let activeTool = 0;

function createToolbar() {
    const toolbar = document.getElementById('toolbar');
    if (!toolbar) return;

    // Tool buttons
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
        if (window._bridge) {
            window._bridge.set_brush_radius(val);
        }
    });

    toolbar.appendChild(sliderLabel);
    toolbar.appendChild(slider);
}

function selectTool(id) {
    activeTool = (activeTool === id) ? 0 : id;
    if (window._bridge) {
        window._bridge.set_tool(activeTool);
    }
    updateButtons();
}

function updateButtons() {
    document.querySelectorAll('.tool-btn').forEach(btn => {
        const toolId = parseInt(btn.dataset.toolId);
        btn.classList.toggle('active', toolId === activeTool);
    });
}

// Listen for keyboard tool shortcuts (synced with bridge.rs)
window.addEventListener('keydown', (e) => {
    const keyMap = { '1': 1, '2': 2, '3': 3, '4': 4, '5': 5, '6': 6 };
    if (keyMap[e.key] !== undefined) {
        activeTool = keyMap[e.key];
        updateButtons();
    } else if (e.key === 'Escape') {
        activeTool = 0;
        updateButtons();
    }
});

// Initialize when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', createToolbar);
} else {
    createToolbar();
}
