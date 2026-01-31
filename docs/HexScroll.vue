<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue';

const props = defineProps({
  scrollY: { type: Number, default: 0 },
  hexagonFactor: { type: Number, default: 1 },
  intensity: { type: Number, default: 1.8 }, // Increased intensity
  density: { type: Number, default: 2.5 }, // Increased density
  excludeArea: { type: Function, default: null },
});

const canvasRef = ref<HTMLCanvasElement | null>(null);
let ctx: CanvasRenderingContext2D | null = null;
let animationFrameId: number | null = null;
let width = 0;
let height = 0;

const nodes = ref<any[]>([]);
const NODE_COUNT = 100; // Increased node count

function resizeCanvas() {
  if (!canvasRef.value) return;
  // Make canvas slightly larger than container for edge-to-edge coverage
  width = canvasRef.value.width = canvasRef.value.offsetWidth * 1.2;
  height = canvasRef.value.height = canvasRef.value.offsetHeight * 1.2;
}

function initNodes() {
  nodes.value = [];
  const nodeCount = Math.floor(NODE_COUNT * props.density);
  for (let i = 0; i < nodeCount; i++) {
    let angle = Math.random() * Math.PI * 2;
    let radius = Math.random() * Math.min(width, height) * 0.5 * props.hexagonFactor + 80;
    let x = width / 2 + Math.cos(angle) * radius;
    let y = height / 2 + Math.sin(angle) * radius;
    nodes.value.push({
      x,
      y,
      vx: (Math.random() - 0.5) * 0.7 * props.intensity, // Faster movement
      vy: (Math.random() - 0.5) * 0.7 * props.intensity, // Faster movement
      size: 2 + Math.random() * 3 * props.intensity, // Variable node sizes
      color: Math.random() > 0.7 ? '#60a5fa' : '#aeefff', // Mix of colors
    });
  }
}

function animate() {
  if (!ctx || !canvasRef.value) return;
  ctx.clearRect(0, 0, width, height);
  
  // Animate nodes
  for (let node of nodes.value) {
    node.x += node.vx;
    node.y += node.vy;
    // Bounce off edges with slight randomization
    if (node.x < 0 || node.x > width) {
      node.vx *= -1.02;
      node.vx += (Math.random() - 0.5) * 0.2;
    }
    if (node.y < 0 || node.y > height) {
      node.vy *= -1.02;
      node.vy += (Math.random() - 0.5) * 0.2;
    }
    // Optionally exclude center area with hexagonFactor
    if (props.excludeArea && props.excludeArea(node.x, node.y, width / 2, height / 2, Math.min(width, height) * 0.35 * props.hexagonFactor)) {
      node.vx *= -1.05; 
      node.vy *= -1.05;
      node.vx += (Math.random() - 0.5) * 0.3;
      node.vy += (Math.random() - 0.5) * 0.3;
    }
  }
  
  // Draw connections with improved visuals
  for (let i = 0; i < nodes.value.length; i++) {
    for (let j = i + 1; j < nodes.value.length; j++) {
      let a = nodes.value[i], b = nodes.value[j];
      let dist = Math.hypot(a.x - b.x, a.y - b.y);
      let maxDist = 180 * props.intensity; // Increased connection distance
      
      if (dist < maxDist) {
        ctx.save();
        let alpha = 0.15 + 0.35 * (1 - dist / maxDist);
        ctx.globalAlpha = alpha;
        
        // Create gradient for connections
        const gradient = ctx.createLinearGradient(a.x, a.y, b.x, b.y);
        gradient.addColorStop(0, a.color || '#aeefff');
        gradient.addColorStop(1, b.color || '#aeefff');
        ctx.strokeStyle = gradient;
        
        ctx.lineWidth = Math.max(0.5, 1.5 * (1 - dist / maxDist));
        ctx.beginPath();
        ctx.moveTo(a.x, a.y);
        ctx.lineTo(b.x, b.y);
        ctx.stroke();
        ctx.restore();
      }
    }
  }
  
  // Draw nodes with glow effect
  for (let node of nodes.value) {
    ctx.save();
    ctx.globalAlpha = 0.85;
    ctx.beginPath();
    ctx.arc(node.x, node.y, node.size, 0, Math.PI * 2);
    ctx.fillStyle = node.color || '#aeefff';
    ctx.shadowColor = node.color || '#60a5fa';
    ctx.shadowBlur = 12;
    ctx.fill();
    
    // Add extra glow for some nodes
    if (Math.random() > 0.7) {
      ctx.globalAlpha = 0.4;
      ctx.beginPath();
      ctx.arc(node.x, node.y, node.size * 2, 0, Math.PI * 2);
      ctx.fillStyle = 'rgba(174, 239, 255, 0.2)';
      ctx.fill();
    }
    
    ctx.restore();
  }
  
  animationFrameId = requestAnimationFrame(animate);
}

function start() {
  if (!canvasRef.value) return;
  ctx = canvasRef.value.getContext('2d');
  resizeCanvas();
  initNodes();
  animate();
}

function stop() {
  if (animationFrameId) cancelAnimationFrame(animationFrameId);
}

onMounted(() => {
  start();
  window.addEventListener('resize', resizeCanvas);
});

onUnmounted(() => {
  stop();
  window.removeEventListener('resize', resizeCanvas);
});

watch(() => [props.scrollY, props.hexagonFactor, props.intensity, props.density], () => {
  resizeCanvas();
  initNodes();
}, { deep: true });
</script>

<template>
  <div class="absolute inset-0 w-full h-full overflow-hidden z-10">
    <canvas ref="canvasRef" class="absolute inset-0 w-full h-full pointer-events-none" />
  </div>
</template>

<style scoped>
canvas {
  background: transparent;
  display: block;
  width: 100%;
  height: 100%;
  filter: blur(0.5px) contrast(1.2) brightness(1.1);
  transform: scale(1.2); /* Slightly larger to ensure full coverage */
}
</style>
