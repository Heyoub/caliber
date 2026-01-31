<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';

const props = defineProps<{
  intensity?: number;
  count?: number;
}>();

const canvas = ref<HTMLCanvasElement | null>(null);
let ctx: CanvasRenderingContext2D | null = null;
let nodes: Array<{x: number; y: number; vx: number; vy: number}> = [];
let animationFrame: number;

function initNodes() {
  const nodeCount = props.count || Math.max(100, Math.floor((window.innerWidth * window.innerHeight) / 10000)); // Scale with screen size
  nodes = [];
  for (let i = 0; i < nodeCount; i++) {
    // Place nodes avoiding the center
    let x, y;
    do {
      x = Math.random() * window.innerWidth;
      y = Math.random() * window.innerHeight;
    } while (isInCenter(x, y));

    nodes.push({
      x,
      y,
      vx: (Math.random() - 0.5) * 1.2, // Slightly slower for smoother movement
      vy: (Math.random() - 0.5) * 1.2
    });
  }
}

function isInCenter(x: number, y: number) {
  const centerX = window.innerWidth / 2;
  const centerY = window.innerHeight / 2;
  const dx = x - centerX;
  const dy = y - centerY;
  const distance = Math.sqrt(dx * dx + dy * dy);
  return distance < Math.min(window.innerWidth, window.innerHeight) * 0.25; // Increased center avoidance
}

function applyRepulsion(node: {x: number; y: number; vx: number; vy: number}) {
  const centerX = window.innerWidth / 2;
  const centerY = window.innerHeight / 2;
  const dx = node.x - centerX;
  const dy = node.y - centerY;
  const distance = Math.sqrt(dx * dx + dy * dy);
  const repulsionRadius = Math.min(window.innerWidth, window.innerHeight) * 0.3; // Increased repulsion radius
  
  if (distance < repulsionRadius) {
    const force = (1 - distance / repulsionRadius) * 0.8; // Stronger repulsion
    node.vx += (dx / distance) * force;
    node.vy += (dy / distance) * force;
  }
}

function drawNode(x: number, y: number) {
  if (!ctx) return;
  ctx.beginPath();
  ctx.arc(x, y, 2, 0, Math.PI * 2);
  ctx.fillStyle = 'rgba(45, 212, 191, 0.6)'; // More transparent nodes
  ctx.fill();
}

function drawConnection(x1: number, y1: number, x2: number, y2: number, distance: number) {
  if (!ctx) return;
  const maxDistance = 250; // Increased for more connections
  const opacity = 1 - distance / maxDistance;
  if (opacity <= 0) return;
  
  ctx.beginPath();
  ctx.moveTo(x1, y1);
  ctx.lineTo(x2, y2);
  ctx.strokeStyle = `rgba(34, 211, 238, ${opacity * 0.3})`; // More transparent connections
  ctx.lineWidth = opacity;
  ctx.stroke();
}

function animate() {
  if (!canvas.value || !ctx) return;
  
  ctx.clearRect(0, 0, canvas.value.width, canvas.value.height);
  
  // Update node positions
  nodes.forEach(node => {
    // Apply center repulsion
    applyRepulsion(node);
    
    // Update position
    node.x += node.vx;
    node.y += node.vy;
    
    // Add slight randomness to movement
    node.vx += (Math.random() - 0.5) * 0.08;
    node.vy += (Math.random() - 0.5) * 0.08;
    
    // Dampen velocity
    node.vx *= 0.98;
    node.vy *= 0.98;
    
    // Bounce off edges with some padding
    const padding = 20;
    if (node.x < padding || node.x > canvas.value!.width - padding) {
      node.vx *= -1;
      node.x = Math.max(padding, Math.min(canvas.value!.width - padding, node.x));
    }
    if (node.y < padding || node.y > canvas.value!.height - padding) {
      node.vy *= -1;
      node.y = Math.max(padding, Math.min(canvas.value!.height - padding, node.y));
    }
  });
  
  // Draw connections first
  nodes.forEach((node1, i) => {
    nodes.slice(i + 1).forEach(node2 => {
      const dx = node2.x - node1.x;
      const dy = node2.y - node1.y;
      const distance = Math.sqrt(dx * dx + dy * dy);
      
      if (distance < 250) { // Increased connection distance
        drawConnection(node1.x, node1.y, node2.x, node2.y, distance);
      }
    });
  });
  
  // Draw nodes on top
  nodes.forEach(node => drawNode(node.x, node.y));
  
  animationFrame = requestAnimationFrame(animate);
}

function resizeCanvas() {
  if (!canvas.value) return;
  canvas.value.width = window.innerWidth;
  canvas.value.height = window.innerHeight;
  initNodes();
}

onMounted(() => {
  if (!canvas.value) return;
  ctx = canvas.value.getContext('2d');
  if (!ctx) return;
  
  resizeCanvas();
  window.addEventListener('resize', resizeCanvas);
  animate();
});

onUnmounted(() => {
  window.removeEventListener('resize', resizeCanvas);
  cancelAnimationFrame(animationFrame);
});
</script>

<template>
  <canvas 
    ref="canvas"
    class="neural-animation"
    :style="{ opacity: intensity }"
  />
</template>

<style scoped>
.neural-animation {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}
</style>