<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';

const props = defineProps<{
  intensity?: number;
  density?: number; // Controls the density of the grid lines
  color?: string; // Primary color theme
}>();

const canvas = ref<HTMLCanvasElement | null>(null);
let ctx: CanvasRenderingContext2D | null = null;
let animationFrame: number;

// Grid properties
const gridPoints: Array<{
  x: number; 
  y: number; 
  opacity: number;
  velocityX?: number;  // For physics movement
  velocityY?: number;  // For physics movement
  rotation?: number;   // For spinning effect
  rotationSpeed?: number; // How fast it spins
}> = [];

const gridLines: Array<{
  x1: number; 
  y1: number; 
  x2: number; 
  y2: number; 
  opacity: number; 
  thickness: number; 
  color: string;
  isDynamic?: boolean; // Whether this line moves
  centerX?: number;    // Center point for rotating lines
  centerY?: number;
  radius?: number;     // Distance from center
  angle?: number;      // Current angle
  rotationSpeed?: number; // How fast it rotates
}> = [];

// Traveling pulses
interface Pulse {
  lineIndex: number;  // Which line it's on
  position: number;   // 0-1 position along the line
  speed: number;      // How fast it travels
  direction: number;  // 1 or -1 (forward/backward)
  color: string;      // Pulse color
  active: boolean;    // Whether pulse is still active
}
const pulses: Pulse[] = [];
let lastPulseTime = 0;

// Create the grid points
function createGrid() {
  if (!canvas.value) return;
  
  const width = canvas.value.width;
  const height = canvas.value.height;
  
  // Clear previous grid
  gridPoints.length = 0;
  gridLines.length = 0;
  
  // Calculate the cell size based on density prop (default to 100px)
  const cellSize = props.density ? 800 / props.density : 100;
  
  // Define center area to avoid (for concentrating nodes on the edges)
  const centerX = width / 2;
  const centerY = height / 2;
  const centerWidth = width * 0.5; // Center 50% width to avoid
  const centerHeight = height * 0.4; // Center 40% height to avoid
  
  // Create grid points - more concentrated on edges
  for (let x = 0; x <= width + cellSize; x += cellSize) {
    for (let y = 0; y <= height + cellSize; y += cellSize) {
      // Skip some points in the center area to create a less dense center
      const distanceFromCenterX = Math.abs(x - centerX);
      const distanceFromCenterY = Math.abs(y - centerY);
      
      // If point is in the center area, only add it with 30% probability
      const isInCenter = distanceFromCenterX < centerWidth / 2 && distanceFromCenterY < centerHeight / 2;
      if (isInCenter && Math.random() > 0.3) {
        continue;
      }
      
      // Add some random offset to make it less uniform
      const offsetX = (Math.random() - 0.5) * cellSize * 0.3;
      const offsetY = (Math.random() - 0.5) * cellSize * 0.3;
      
      // Points at the edges have higher opacity
      let opacity = 0.1 + Math.random() * 0.4;
      if (!isInCenter) {
        opacity += 0.2; // Boost opacity for edge nodes
      }
      
      gridPoints.push({
        x: x + offsetX,
        y: y + offsetY,
        opacity: opacity,
        velocityX: 0,
        velocityY: 0,
        rotation: Math.random() * Math.PI * 2,
        rotationSpeed: 0 // Initially no rotation
      });
    }
  }
  
  // Create grid lines with ForgeStack color palette (less teal)
  // Primary colors from ForgeStack - reduced teal presence (only 20% of original)
  const primaryColors = ['#805AD5', '#D53F8C80', '#4FD1C5', '#805AD5', '#D53F8C80', '#4FD1C5', '#805AD5', '#4FD1C5', '#805AD5'];
  const defaultColor = '#805AD5'; // Default is now purple instead of teal
  const lineColor = props.color || defaultColor;
  
  // Create horizontal lines
  for (let y = 0; y <= height + cellSize; y += cellSize) {
    // Some random offset for more organic feel
    const offsetY = (Math.random() - 0.5) * cellSize * 0.2;
    
    gridLines.push({
      x1: 0,
      y1: y + offsetY,
      x2: width,
      y2: y + offsetY,
      opacity: 0.1 + Math.random() * 0.3,
      thickness: Math.random() > 0.8 ? 2 : 1, // Occasionally thicker lines
      color: Math.random() > 0.7 ? primaryColors[Math.floor(Math.random() * primaryColors.length)] : lineColor
    });
  }
  
  // Create vertical lines
  for (let x = 0; x <= width + cellSize; x += cellSize) {
    // Some random offset for more organic feel
    const offsetX = (Math.random() - 0.5) * cellSize * 0.2;
    
    gridLines.push({
      x1: x + offsetX,
      y1: 0,
      x2: x + offsetX,
      y2: height,
      opacity: 0.1 + Math.random() * 0.3,
      thickness: Math.random() > 0.8 ? 2 : 1,
      color: Math.random() > 0.7 ? primaryColors[Math.floor(Math.random() * primaryColors.length)] : lineColor
    });
  }
  
  // Create diagonal lines (fewer of these, but dynamic)
  for (let i = 0; i < Math.floor(width / cellSize) * 0.5; i++) {
    // Choose a random point around the canvas to place the line's center
    const centerX = Math.random() * width;
    const centerY = Math.random() * height;
    
    // Create a spinning line
    const radius = cellSize * (1 + Math.random() * 3);
    const angle = Math.random() * Math.PI * 2;
    const rotationSpeed = (Math.random() * 0.001) + 0.0005; // Between 0.0005 and 0.0015
    
    gridLines.push({
      x1: centerX + Math.cos(angle) * radius,
      y1: centerY + Math.sin(angle) * radius,
      x2: centerX + Math.cos(angle + Math.PI) * radius,
      y2: centerY + Math.sin(angle + Math.PI) * radius,
      opacity: 0.05 + Math.random() * 0.2,
      thickness: Math.random() > 0.8 ? 2 : 1,
      color: Math.random() > 0.5 ? primaryColors[Math.floor(Math.random() * primaryColors.length)] : lineColor,
      isDynamic: true,
      centerX: centerX,
      centerY: centerY,
      radius: radius,
      angle: angle,
      rotationSpeed: rotationSpeed * (Math.random() > 0.5 ? 1 : -1) // Randomize direction
    });
  }
}

  // Draw a grid point
function drawPoint(point: typeof gridPoints[0]) {
  if (!ctx) return;
  
  // Calculate distance from center for color gradient
  const centerX = canvas.value!.width / 2;
  const centerY = canvas.value!.height / 2;
  const dx = point.x - centerX;
  const dy = point.y - centerY;
  const distance = Math.sqrt(dx * dx + dy * dy);
  const maxDistance = Math.sqrt(centerX * centerX + centerY * centerY);
  const ratio = distance / maxDistance;
  
  // Color gradient based on distance from center
  let pointColor;
  if (ratio < 0.3) {
    // Center points - more pink/purple
    pointColor = ratio < 0.15 ? '#D53F8C' : '#805AD5';
  } else if (ratio < 0.7) {
    // Mid points - mix of colors
    pointColor = ratio < 0.5 ? '#805AD5' : '#4FD1C5';
  } else {
    // Edge points - occasional teal (20% of original)
    pointColor = Math.random() > 0.8 ? '#805AD5' : '#805AD5';
  }
  
  const size = 1.5 + (ratio * 1); // Slightly larger nodes at the edges
  
  ctx.save();
  
  // Translate to the point's position
  ctx.translate(point.x, point.y);
  
  // Apply rotation if the point has been hit
  if (point.rotation !== undefined) {
    ctx.rotate(point.rotation);
  }
  
  // Draw the point (now centered at origin due to translate)
  ctx.beginPath();
  ctx.arc(0, 0, size, 0, Math.PI * 2);
  ctx.fillStyle = `${pointColor}${Math.floor(point.opacity * 255).toString(16).padStart(2, '0')}`;
  ctx.fill();
  
  ctx.restore();
}

// Draw a grid line
function drawLine(x1: number, y1: number, x2: number, y2: number, opacity: number, thickness: number, color: string) {
  if (!ctx) return;
  
  ctx.beginPath();
  ctx.moveTo(x1, y1);
  ctx.lineTo(x2, y2);
  ctx.strokeStyle = color.replace(')', `, ${opacity})`).replace('rgb', 'rgba');
  ctx.lineWidth = thickness;
  ctx.stroke();
}

// Create a new pulse that travels along a grid line
function createPulse() {
  if (gridLines.length === 0) return;
  
  // Choose a random line
  const lineIndex = Math.floor(Math.random() * gridLines.length);
  
  // Random color from gradient
  const pulseColors = ['#805AD5', '#D53F8C', '#4FD1C5', '#805AD5'];
  const color = pulseColors[Math.floor(Math.random() * pulseColors.length)];
  
  // Create pulse
  pulses.push({
    lineIndex,
    position: Math.random() > 0.5 ? 0 : 1, // Start from either end
    speed: 0.001 + Math.random() * 0.002, // Random speed
    direction: Math.random() > 0.5 ? 1 : -1, // Random direction
    color,
    active: true
  });
}

// Check for pulse collisions
function checkPulseCollisions() {
  for (let i = 0; i < pulses.length; i++) {
    if (!pulses[i].active) continue;
    
    for (let j = i + 1; j < pulses.length; j++) {
      if (!pulses[j].active) continue;
      
      // Only check pulses on the same line
      if (pulses[i].lineIndex === pulses[j].lineIndex) {
        const dist = Math.abs(pulses[i].position - pulses[j].position);
        
        // If pulses are close and moving in opposite directions
        if (dist < 0.05 && pulses[i].direction !== pulses[j].direction) {
          // Create a flash effect at the collision point
          createFlash(
            pulses[i].lineIndex,
            (pulses[i].position + pulses[j].position) / 2
          );
          
          // Deactivate both pulses
          pulses[i].active = false;
          pulses[j].active = false;
        }
      }
    }
  }
}

// Create a flash effect at a point on a line
function createFlash(lineIndex: number, position: number) {
  if (!ctx || !canvas.value) return;
  
  const line = gridLines[lineIndex];
  if (!line) return;
  
  // Calculate position on the line
  const x = line.x1 + (line.x2 - line.x1) * position;
  const y = line.y1 + (line.y2 - line.y1) * position;
  
  // Draw flash
  const gradient = ctx.createRadialGradient(x, y, 0, x, y, 20);
  gradient.addColorStop(0, 'rgba(255, 255, 255, 0.8)');
  gradient.addColorStop(1, 'rgba(255, 255, 255, 0)');
  
  ctx.beginPath();
  ctx.fillStyle = gradient;
  ctx.arc(x, y, 20, 0, Math.PI * 2);
  ctx.fill();
}

// Check if a line segment intersects with a point (with radius)
function linePointCollision(line: typeof gridLines[0], point: typeof gridPoints[0], pointRadius: number): boolean {
  // Calculate the closest point on the line to the circle
  const x1 = line.x1;
  const y1 = line.y1;
  const x2 = line.x2;
  const y2 = line.y2;
  
  // Calculate the line segment length squared
  const lengthSquared = (x2 - x1) * (x2 - x1) + (y2 - y1) * (y2 - y1);
  if (lengthSquared === 0) return false; // Line segment is actually a point
  
  // Calculate the projection of the point onto the line
  const t = Math.max(0, Math.min(1, ((point.x - x1) * (x2 - x1) + (point.y - y1) * (y2 - y1)) / lengthSquared));
  
  // Find the closest point on the line segment
  const closestX = x1 + t * (x2 - x1);
  const closestY = y1 + t * (y2 - y1);
  
  // Check if the distance is less than the point radius
  const distance = Math.sqrt((point.x - closestX) * (point.x - closestX) + (point.y - closestY) * (point.y - closestY));
  return distance < pointRadius + line.thickness;
}

// Handle collisions between dynamic lines and points
function handleCollisions() {
  const pointRadius = 3; // Average point radius
  
  // Check each dynamic line against each point
  gridLines.forEach(line => {
    if (!line.isDynamic) return;
    
    gridPoints.forEach(point => {
      if (linePointCollision(line, point, pointRadius)) {
        // Calculate normal vector from line to point
        const x1 = line.x1;
        const y1 = line.y1;
        const x2 = line.x2;
        const y2 = line.y2;
        
        // Find the closest point on the line segment to the point
        const lengthSquared = (x2 - x1) * (x2 - x1) + (y2 - y1) * (y2 - y1);
        const t = Math.max(0, Math.min(1, ((point.x - x1) * (x2 - x1) + (point.y - y1) * (y2 - y1)) / lengthSquared));
        const closestX = x1 + t * (x2 - x1);
        const closestY = y1 + t * (y2 - y1);
        
        // Calculate normal vector (from line to point)
        const nx = point.x - closestX;
        const ny = point.y - closestY;
        const len = Math.sqrt(nx * nx + ny * ny);
        
        if (len > 0) {
          // Normalize the normal vector
          const normalX = nx / len;
          const normalY = ny / len;
          
          // Set point velocity based on the line's movement
          const lineVectorX = x2 - x1;
          const lineVectorY = y2 - y1;
          const lineDirX = lineVectorX / Math.sqrt(lineVectorX * lineVectorX + lineVectorY * lineVectorY);
          const lineDirY = lineVectorY / Math.sqrt(lineVectorX * lineVectorX + lineVectorY * lineVectorY);
          
          // Apply gentle push away from the line (perpendicular to line direction)
          point.velocityX = normalX * 0.1;
          point.velocityY = normalY * 0.1;
          
          // Start the point spinning
          point.rotationSpeed = 0.02 + Math.random() * 0.03;
        }
      }
    });
  });
}

// Animation loop
function animate() {
  if (!canvas.value || !ctx) return;
  
  ctx.clearRect(0, 0, canvas.value.width, canvas.value.height);
  
  // Animation time for wave effects
  const time = Date.now() / 1000;
  const currentTime = Date.now();
  const centerX = canvas.value.width / 2;
  const centerY = canvas.value.height / 2;
  
  // Update dynamic line positions - rotate them around their centers
  gridLines.forEach(line => {
    if (line.isDynamic && line.centerX !== undefined && line.centerY !== undefined && 
        line.radius !== undefined && line.angle !== undefined && line.rotationSpeed !== undefined) {
      
      // Update angle
      line.angle += line.rotationSpeed;
      
      // Calculate new endpoints
      line.x1 = line.centerX + Math.cos(line.angle) * line.radius;
      line.y1 = line.centerY + Math.sin(line.angle) * line.radius;
      line.x2 = line.centerX + Math.cos(line.angle + Math.PI) * line.radius;
      line.y2 = line.centerY + Math.sin(line.angle + Math.PI) * line.radius;
    }
  });
  
  // Check for collisions between dynamic lines and points
  handleCollisions();
  
  // Update point positions and rotations
  gridPoints.forEach(point => {
    // Apply velocity if the point is moving
    if (point.velocityX !== undefined && point.velocityY !== undefined) {
      point.x += point.velocityX;
      point.y += point.velocityY;
      
      // Dampen velocity (gradually slow down)
      point.velocityX *= 0.98;
      point.velocityY *= 0.98;
      
      // Stop very small movements to prevent endless tiny motion
      if (Math.abs(point.velocityX) < 0.001) point.velocityX = 0;
      if (Math.abs(point.velocityY) < 0.001) point.velocityY = 0;
    }
    
    // Update rotation if spinning
    if (point.rotation !== undefined && point.rotationSpeed !== undefined) {
      point.rotation += point.rotationSpeed;
      
      // Dampen rotation speed
      point.rotationSpeed *= 0.99;
      
      // Stop very slow rotation
      if (Math.abs(point.rotationSpeed!) < 0.001) point.rotationSpeed = 0;
    }
  });
  
  // Check if we should create a new pulse (every 3-4 seconds)
  if (currentTime - lastPulseTime > 3000 + Math.random() * 1000) {
    if (pulses.filter(p => p.active).length < 3) { // Limit active pulses
      createPulse();
      lastPulseTime = currentTime;
    }
  }
  
  // Update and check pulses
  pulses.forEach(pulse => {
    if (!pulse.active) return;
    
    // Update position
    pulse.position += pulse.speed * pulse.direction;
    
    // If pulse reaches end of line, deactivate it
    if (pulse.position <= 0 || pulse.position >= 1) {
      pulse.active = false;
    }
  });
  
  // Check for collisions
  checkPulseCollisions();
  
  // Remove inactive pulses when we have too many
  if (pulses.length > 20) {
    const activeIndex = pulses.findIndex(p => !p.active);
    if (activeIndex >= 0) {
      pulses.splice(activeIndex, 1);
    }
  }
  
  // Draw the grid lines first
  gridLines.forEach((line, index) => {
    // Calculate distance from center for both endpoints
    const dx1 = line.x1 - centerX;
    const dy1 = line.y1 - centerY;
    const distance1 = Math.sqrt(dx1 * dx1 + dy1 * dy1);
    
    const dx2 = line.x2 - centerX;
    const dy2 = line.y2 - centerY;
    const distance2 = Math.sqrt(dx2 * dx2 + dy2 * dy2);
    
    // Average distance from center
    const avgDistance = (distance1 + distance2) / 2;
    const maxDistance = Math.sqrt(centerX * centerX + centerY * centerY);
    const ratio = avgDistance / maxDistance;
    
    // Create wave-like pulsing from edges to center
    const waveOffset = ratio * 2 * Math.PI;
    const wavePhase = (time * 0.5) + waveOffset;
    const waveAmount = Math.sin(wavePhase) * 0.15;
    
    // Make pink lines more transparent
    let finalOpacity = Math.max(0.05, Math.min(0.8, line.opacity + waveAmount));
    if (line.color.includes("D53F8C")) {
      finalOpacity *= 0.6; // Reduce pink opacity by 40%
    }
    
    drawLine(
      line.x1, 
      line.y1, 
      line.x2, 
      line.y2, 
      finalOpacity * (ratio * 0.5 + 0.5), // Gradually fade lines toward center
      line.thickness,
      line.color
    );
    
    // Draw pulses on this line
    pulses.forEach(pulse => {
      if (pulse.active && pulse.lineIndex === index) {
        // Calculate position along the line
        const x = line.x1 + (line.x2 - line.x1) * pulse.position;
        const y = line.y1 + (line.y2 - line.y1) * pulse.position;
        
        // Draw pulse glow
        ctx!.beginPath();
        const gradient = ctx!.createRadialGradient(x, y, 0, x, y, 15);
        gradient.addColorStop(0, pulse.color + 'CC'); // Semi-transparent
        gradient.addColorStop(1, pulse.color + '00'); // Fully transparent
        ctx!.fillStyle = gradient;
        ctx!.arc(x, y, 15, 0, Math.PI * 2);
        ctx!.fill();
        
        // Draw pulse core
        ctx!.beginPath();
        ctx!.fillStyle = pulse.color;
        ctx!.arc(x, y, 2, 0, Math.PI * 2);
        ctx!.fill();
      }
    });
  });
  
  // Draw the grid points on top
  gridPoints.forEach(point => {
    // Calculate distance from center
    const dx = point.x - centerX;
    const dy = point.y - centerY;
    const distance = Math.sqrt(dx * dx + dy * dy);
    const maxDistance = Math.sqrt(centerX * centerX + centerY * centerY);
    const ratio = distance / maxDistance;
    
    // Create wave-like pulsing from edges to center
    const waveOffset = ratio * 2 * Math.PI;
    const wavePhase = (time * 0.5) + waveOffset;
    const waveAmount = Math.sin(wavePhase) * 0.2;
    
    // More pronounced animation for points at the edges
    const pulseIntensity = ratio * 0.15 + 0.1;
    const pulseAmount = Math.sin(time + point.x * point.y * 0.001) * pulseIntensity;
    const adjustedOpacity = Math.max(0.05, Math.min(0.9, point.opacity + pulseAmount + waveAmount));
    
    // Update opacity
    point.opacity = adjustedOpacity;
    
    // Draw the point
    drawPoint(point);
  });
  
  animationFrame = requestAnimationFrame(animate);
}

// Resize handler
function resizeCanvas() {
  if (!canvas.value) return;
  
  canvas.value.width = window.innerWidth;
  canvas.value.height = window.innerHeight;
  
  createGrid();
}

// Lifecycle hooks
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
    class="grid-animation"
    :style="{ opacity: intensity || 1 }"
  />
</template>

<style scoped>
.grid-animation {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  z-index: 0;
}
</style>
