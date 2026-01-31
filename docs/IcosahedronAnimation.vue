<template>
  <div ref="containerRef" class="w-full h-full relative"></div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import * as THREE from 'three';

const containerRef = ref<HTMLDivElement | null>(null);
let renderer: THREE.WebGLRenderer | null = null;
let scene: THREE.Scene | null = null;
let camera: THREE.PerspectiveCamera | null = null;
let mesh: THREE.Mesh | null = null;
let animationFrameId: number | null = null;

function init() {
  if (!containerRef.value) return;

  // Scene
  scene = new THREE.Scene();

  // Camera
  camera = new THREE.PerspectiveCamera(75, containerRef.value.clientWidth / containerRef.value.clientHeight, 0.1, 1000);
  camera.position.z = 2;

  // Renderer
  renderer = new THREE.WebGLRenderer({ alpha: true, antialias: true }); // Enable alpha for transparency
  renderer.setSize(containerRef.value.clientWidth, containerRef.value.clientHeight);
  renderer.setPixelRatio(window.devicePixelRatio);
  containerRef.value.appendChild(renderer.domElement);

  // Geometry - Icosahedron
  const geometry = new THREE.IcosahedronGeometry(0.7, 0); // Use radius 0.7

  // --- Vertex Colors --- 
  const colors: number[] = [];
  const color1 = new THREE.Color(0x8A2BE2); // BlueViolet (Purple)
  const color2 = new THREE.Color(0x00FF7F); // SpringGreen
  const positionAttribute = geometry.attributes.position;

  for (let i = 0; i < positionAttribute.count; i++) {
    // Simple alternation or based on position (e.g., y-coordinate)
    const y = positionAttribute.getY(i);
    const color = y > 0 ? color1 : color2; // Green top, Purple bottom
    colors.push(color.r, color.g, color.b);
  }
  geometry.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3));
  // --- End Vertex Colors ---

  // Material - Use vertex colors
  const material = new THREE.MeshPhongMaterial({
    // color: 0xffffff, // Base color is less important now
    shininess: 100,
    specular: 0x555555,
    vertexColors: true // Enable vertex colors
  });

  // Mesh
  mesh = new THREE.Mesh(geometry, material);
  scene.add(mesh);

  // Lighting
  const ambientLight = new THREE.AmbientLight(0xffffff, 0.5); // Soft ambient light
  scene.add(ambientLight);
  const pointLight = new THREE.PointLight(0xffffff, 1, 100);
  pointLight.position.set(2, 3, 4);
  scene.add(pointLight);

  // Start animation
  animate();

  // Handle resize
  window.addEventListener('resize', onWindowResize);
}

function animate() {
  if (!renderer || !scene || !camera || !mesh) return;

  animationFrameId = requestAnimationFrame(animate);

  // Rotation
  mesh.rotation.x += 0.005;
  mesh.rotation.y += 0.005;

  renderer.render(scene, camera);
}

function onWindowResize() {
  if (!camera || !renderer || !containerRef.value) return;

  camera.aspect = containerRef.value.clientWidth / containerRef.value.clientHeight;
  camera.updateProjectionMatrix();
  renderer.setSize(containerRef.value.clientWidth, containerRef.value.clientHeight);
}

let cleanupFunction: (() => void) | undefined;

onMounted(() => {
  if (containerRef.value) {
    init();
  }
});

onUnmounted(() => {
  if (animationFrameId) {
    cancelAnimationFrame(animationFrameId);
  }
  if (renderer && renderer.domElement && containerRef.value && containerRef.value.contains(renderer.domElement)) {
    containerRef.value.removeChild(renderer.domElement);
  }
  window.removeEventListener('resize', onWindowResize);
});

</script>

<style scoped>
div {
  width: 100%;
  height: 100%;
  min-height: 200px; 
  min-width: 200px;  
}
</style>
