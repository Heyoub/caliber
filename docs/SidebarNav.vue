<template>
<nav class="flex flex-col bg-slate-700 min-h-screen px-1.5 pt-8 relative z-50">
  <!-- ForgeStack Button -->
    <GlowPressButton
      id="forgestack-button"
      ref="buttonRef"
      color="glassy"
      size="lg"
      class="mb-1 w-full"
      :aria-expanded="isDropdownOpen.toString()"
      aria-haspopup="true"
      :forcePressed="isDropdownOpen"
      @click="handleButtonClick"
      data-dropdown-trigger="forgestack"
    >
    <span class="flex flex-row items-center justify-center gap-0">
        <!-- Hamburger Icon (moved first) -->
        <span class="h-8 w-3 flex flex-col mt-[1rem] ml-2 items-center justify-center gap-[0.375rem]">
          <span ref="line1Ref" class="w-[1.66rem] h-[0.33rem] bg-slate-100/90 rounded-full transition-all duration-300 origin-center" data-menu-line="forgestack-1"></span>
          <span ref="line2Ref" class="w-[1.66rem] h-[0.33rem] bg-slate-100/90 rounded-full transition-all duration-300 origin-center" data-menu-line="forgestack-2"></span>
        </span>
        <!-- Logo Image (replaces text) -->
        <img src="/IMG/AXLG.svg" alt="ForgeStack Logo" class="h-14 w-auto opacity-90" />
        <span class="mt-[1rem] ml-[-1.75rem] text-md font-semibold text-white tracking-wider">orgeStack</span>
      </span>
    </GlowPressButton>

    <!-- ForgeStack Dropdown Menu -->
    <div
      id="forgestack-dropdown"
      ref="dropdownRef"
      class="fixed w-[10rem] h-[auto] bg-slate-700/90 backdrop-blur-xl border border-white/10 border-solid rounded-lg shadow-[0_0.25rem_0.25rem_rgba(41,171,226,0.15)] overflow-hidden opacity-0 translate-y-2 pointer-events-none transition-all duration-300 before:absolute before:inset-0 before:bg-gradient-to-b before:from-white/5 before:to-transparent before:pointer-events-none after:absolute after:inset-0 after:bg-gradient-radial after:from-blue-400/10 after:to-transparent after:opacity-0 after:transition-opacity after:duration-500"
      data-dropdown="forgestack"
      :data-state="isDropdownOpen ? 'open' : 'closed'"
      style="left: 10vw; top: 5vh;"
    >
      <!-- Laser effect layer -->
      <div ref="laserEffectRef" id="forgestack-laser-effect" class="absolute inset-0 pointer-events-none opacity-10 mix-blend-plus-lighter transform-gpu hover:opacity-20 transition-opacity duration-500"></div>

      <!-- Links -->
      <div class="relative z-10 py-1">
        <div v-for="(parentLink, index) in forgestackLinks" :key="parentLink.text" class="category-block">
          <!-- Add separator before all categories except the first one -->
          <hr v-if="index > 0" class="border-t border-white/10 border-solid my-1 mx-4" />

          <!-- Render Parent Category Header with Icon -->
          <div v-if="parentLink.children && parentLink.children.length > 0" class="flex items-center gap-2 px-[1rem] pt-[0.75rem] pb-[0.25rem] text-xs font-semibold text-white/60 uppercase tracking-wider">
            <component :is="parentLink.icon" class="w-4 h-4 opacity-70" />
            <span>{{ parentLink.text }}</span>
          </div>

          <!-- Render Child Links -->
          <template v-if="parentLink.children">
            <a
              v-for="childLink in parentLink.children"
              :key="childLink.href"
              :href="childLink.href"
              @click="closeMenu"

              class="block px-[1rem] py-[0.5rem] text-white/80 hover:text-white text-left
                     hover:bg-blue-400/5 transition-all duration-300
                     relative group/link text-sm"
              @mousemove="handleMouseMove"
              :aria-current="currentPath === childLink.href ? 'page' : undefined"
            >
              <span class="relative z-10">{{ childLink.text }}</span>
              <span class="absolute bottom-0 left-1/2 right-1/2 h-px bg-white/80
                         group-hover/link:left-[1rem] group-hover/link:right-[1rem]
                         transition-all duration-300
                         shadow-[0_0.5rem_1.5rem_rgba(79,209,197,0.5)]
                         after:absolute after:inset-0 after:bg-white/20 after:blur-sm after:opacity-0 after:group-hover/link:opacity-100 after:transition-opacity">
              </span>
            </a>
          </template>
        </div>
      </div>
    </div>
    
    <div class="flex-grow min-h-0"></div>
    <!-- Footer content -->
    <div class="mt-auto px-0 space-y-3 flex-shrink-0 relative bottom-0 z-10 w-full">
      <!-- Combined company description -->
      <div class="px-2 py-3 rounded-lg bg-white/5 backdrop-blur-sm border border-white/10 border-solid mb-3">
        <h3 class="text-s font-semibold mb-2 text-forgestack-teal flex items-center">
          About:
        </h3>
        <p class="text-xs text-white/70 mb-3">
          ForgeStack is small business technology with human-centered design. We streamline workflows and reduce cognitive load.
        </p>
        <p class="text-xs text-white/70 mb-1">
          Built by entrepreneurs, for entrepreneurs—with full data control, elegant simplicity, and no enterprise bloat.
        </p>
      </div>
                
      <!-- Contact info -->
      <div class="px-2 py-2 rounded-lg flex flex-col items-center bg-white/5 backdrop-blur-sm border border-white/10 border-solid">
        <h3 class="text-s font-semibold mb-2 text-forgestack-teal">
          Let's Connect:
        </h3>
        <div class="flex items-center gap-1 text-white/70 mb-2">
          <a href="mailto:info@forgestack.com" class="hover:text-forgestack-primary transition-colors text-xs">
            info@forgestack.com
          </a>
        </div>
        <div class="flex items-center gap-2 text-white/70 mb-2">
          
        </div>
        <!-- Social links -->
      <div class="flex gap-3 px-1">
        <a href="#" class="text-white/60 hover:text-forgestack-teal transition-colors">
          <svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 4s-.7 2.1-2 3.4c1.6 10-9.4 17.3-18 11.6 2.2.1 4.4-.6 6-2C3 15.5.5 9.6 3 5c2.2 2.6 5.6 4.1 9 4-.9-4.2 4-6.6 7-3.8 1.1 0 3-1.2 3-1.2z"></path></svg>
        </a>
        <a href="#" class="text-white/60 hover:text-forgestack-primary transition-colors">
          <svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 8a6 6 0 0 1 6 6v7h-4v-7a2 2 0 0 0-2-2 2 2 0 0 0-2 2v7h-4v-7a6 6 0 0 1 6-6z"></path><rect width="4" height="12" x="2" y="9"></rect><circle cx="4" cy="4" r="2"></circle></svg>
        </a>
        <a href="#" class="text-white/60 hover:text-forgestack-secondary transition-colors">
          <svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4"></path><path d="M9 18c-4.51 2-5-2-7-2"></path></svg>
        </a>
      </div>     
      </div>

      <!-- Legal links -->
      <div class="px-1 pb-4">
        <div class="flex justify-Center gap-4 text-xs mb-6">
          <a href="#" class="text-white/60 hover:text-forgestack-teal transition-colors">
            Privacy
          </a>
          <a href="#" class="text-white/60 hover:text-forgestack-teal transition-colors">
            Terms
          </a>
          <a href="#" class="text-white/60 hover:text-forgestack-teal transition-colors">
            Data
          </a>
        </div>
      
        <div class="flex justify-center text-xs text-white/50">
          {{ new Date().getFullYear() }} ForgeStack
        </div>
      </div>
    </div>
  </nav>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue';
import { useFloating, offset, flip, shift, autoUpdate } from '@floating-ui/vue';
import { useRoute } from 'vue-router';
import GlowPressButton from '../ui/GlowPressButton.vue';
import { Home, Cloud, Server, Briefcase, Lock } from 'lucide-vue-next';

// --- Props / Route --- 
const route = useRoute();
const currentPath = computed(() => route.path);

// --- Refs --- 
const buttonRef = ref<InstanceType<typeof GlowPressButton> | null>(null);
const dropdownRef = ref<HTMLElement | null>(null);
const laserEffectRef = ref<HTMLElement | null>(null);
const line1Ref = ref<HTMLElement | null>(null); // Ref for hamburger line 1
const line2Ref = ref<HTMLElement | null>(null); // Ref for hamburger line 2

// --- Floating UI Setup ---
const { floatingStyles, update } = useFloating(buttonRef, dropdownRef, {
  placement: 'right-start', // Adjust placement as needed
  middleware: [
    offset(10), // Add 10px offset from the button
    flip(),    // Flip placement if it overflows
    shift({ padding: 5 }) // Shift to stay in view, with 5px padding from edges
  ],
  whileElementsMounted: autoUpdate, // Keep position updated automatically
});

// --- State --- 
// Use null to indicate no menu is active, mirroring the original script's logic
const activeMenu = ref<'forgestack' | null>(null); 
const isDropdownOpen = computed(() => activeMenu.value === 'forgestack');

// --- Data ---
const forgestackLinks = [
  { 
    text: 'Menu', 
    href: '#', 
    icon: Home, 
    children: [
      { text: 'Home: ForgeStack', href: '/' },
      { text: 'ChatFS: Business Ai', href: '/ai'},
      { text: 'Strategy: Fractional CTO', href: '/cto'},
      { text: 'Modules', href: '/modules' },    ]
  },
  {
    text: 'Log-in',
    href: '#',
    icon: Lock, // Using lock icon for login
    children: [
      { text: 'Log-in', href: '/forgestack.app/auth' }
    ]
  }
  // Original navigation structure is commented out for future reference
  /*
  { text: 'Home', href: '#', icon: Home, children: [
      { text: 'Home', href: '/' },
      { text: 'Modules', href: '/modules' },
    ]
  },
  {
    text: 'Cloud',
    href: '#',
    icon: Cloud,
    children: [
      { text: 'ForgeStack', href: '/crm' },
      { text: 'Log-in', href: '/forgestack.app/auth' }
    ]
  },
  {
    text: 'On-Premise',
    href: '#',
    icon: Server,
    children: [
      { text: 'On-Premise', href: '/fsop' },
      { text: 'RiO|mini', href: '/rio' },
      { text: 'RiO|link', href: '/riolink' }
    ]
  },
  { text: 'Services', href: '#', icon: Briefcase, children: [
      { text: 'Ai Services', href: '/ai'}
    ]
  }
  */
];

// --- Methods (adapted from original script) ---

function openMenu() {
  if (!buttonRef.value?.$el || !dropdownRef.value || !line1Ref.value || !line2Ref.value) return;
  
  // Calculate position (from original script)
  const buttonRect = buttonRef.value.$el.getBoundingClientRect();
  
  // Position exactly like in the original image - directly to the right
  // Adjust these values to match the exact position in your reference image
  // Removed fixed positioning, handled by Floating UI
  
  // Set state
  activeMenu.value = 'forgestack';
  dropdownRef.value.dataset.state = 'open';
  buttonRef.value.$el.setAttribute('aria-expanded', 'true');
  buttonRef.value.$el.setAttribute('data-menu-open', 'true');

  // Animate hamburger to X
  line1Ref.value.style.transform = 'rotate(45deg) translateY(0.375rem)';
  line2Ref.value.style.transform = 'rotate(-45deg) translateY(-0.375rem)';
  
  // Add mouse move handler when menu opens
  document.addEventListener('mousemove', handleMouseMove);

  // Initialize laser effect
  if (laserEffectRef.value) {
    laserEffectRef.value.style.background = 'none';
    laserEffectRef.value.style.opacity = '0.2';
  }
  
  // Initial update in case the menu starts open or elements shift on load
  // Note: autoUpdate handles ongoing updates, this is for initial render
  if (buttonRef.value && dropdownRef.value) {
     update(); // Perform initial position calculation
  }
}

// Function to handle mouse movement for laser effect with brand colors
const handleMouseMove = (e: MouseEvent) => {
  if (!laserEffectRef.value || !dropdownRef.value || activeMenu.value !== 'forgestack') return;

  const rect = dropdownRef.value.getBoundingClientRect();
  const linkElement = document.elementFromPoint(e.clientX, e.clientY) as HTMLElement;
  const x = e.clientX - rect.left;
  const y = e.clientY - rect.top;

  if (linkElement) {
    // Find the closest link or the element itself if it's a link
    const linkTarget = linkElement.tagName === 'A' ? linkElement : linkElement.closest('a');
    
    if (linkTarget) {
      const linkRect = linkTarget.getBoundingClientRect();
      const textWidth = linkTarget.offsetWidth;
      
      // Simple, subtle teal effect
      laserEffectRef.value.style.background = `
        radial-gradient(
          circle at ${x}px ${y}px,
          rgba(79, 209, 197, 0.2) 0%,
          transparent 40%
        )
      `;
      laserEffectRef.value.style.opacity = '0.8';
    }
  } else {
    // When not hovering over a link, very subtle glow
    laserEffectRef.value.style.background = `
      radial-gradient(
        circle at ${x}px ${y}px,
        rgba(79, 209, 197, 0.1) 0%,
        transparent 40%
      )
    `;
    laserEffectRef.value.style.opacity = '0.2';
  }
};

function closeMenu() {
  if (!buttonRef.value?.$el || !dropdownRef.value || !line1Ref.value || !line2Ref.value || activeMenu.value !== 'forgestack') return;

  // Reset state
  activeMenu.value = null;
  if (dropdownRef.value) dropdownRef.value.dataset.state = 'closed';
  if (buttonRef.value?.$el) {
    buttonRef.value.$el.setAttribute('aria-expanded', 'false');
    buttonRef.value.$el.removeAttribute('data-menu-open');
  }

  // Reset hamburger menu
  if (line1Ref.value) line1Ref.value.style.transform = 'none';
  if (line2Ref.value) line2Ref.value.style.transform = 'none';
  
  // Remove mousemove handler when menu closes
  document.removeEventListener('mousemove', handleMouseMove);
}

function handleButtonClick() {
  if (activeMenu.value === 'forgestack') {
    closeMenu();
  } else {
    // Need to get the button element to pass to openMenu if it relies on event target
    // Since we removed the event, we need to ensure openMenu can work without it
    // or get the reference differently if needed.
    // Assuming openMenu primarily needs buttonRef, which is available.
    openMenu(); 
  }
}

// --- Event Handlers ---

const handleClickOutside = (event: MouseEvent) => {
  // Close if click is outside the button AND the dropdown
  const buttonEl = buttonRef.value?.$el as HTMLElement;
  if (activeMenu.value === 'forgestack' && dropdownRef.value && buttonEl &&
      !dropdownRef.value.contains(event.target as Node) &&
      !buttonEl.contains(event.target as Node)) {
    closeMenu();
  }
};

const handleEscapeKey = (event: KeyboardEvent) => {
  if (event.key === 'Escape' && activeMenu.value === 'forgestack') {
    closeMenu();
  }
};

// --- Lifecycle Hooks ---
onMounted(() => {
  // Add global listeners when component mounts
  document.addEventListener('click', handleClickOutside, true);
  document.addEventListener('keydown', handleEscapeKey);
  // Mouse move is added/removed dynamically when menu opens/closes
});

onUnmounted(() => {
  // Cleanup global listeners when component unmounts
  document.removeEventListener('click', handleClickOutside, true);
  document.removeEventListener('keydown', handleEscapeKey);
  document.removeEventListener('mousemove', handleMouseMove); // Ensure mousemove is cleaned up
  // Close menu explicitly on unmount if open, to clean up states/listeners
  if (activeMenu.value === 'forgestack') {
      closeMenu();
  }
  // Cleanup for autoUpdate is handled by whileElementsMounted
});

// Watcher to ensure DOM refs are ready if needed for initial state 
// (though button click handles opening)
watch(buttonRef, async (newVal) => {
    if (newVal) {
        await nextTick();
        // Can perform actions requiring the button DOM element here if needed on load
    }
});

</script>

<style scoped>
/* Ensure bg-surface is defined globally or replace if necessary */
/* Assuming --background HSL is defined in :root like in style.css */
/*.bg-surface\/80 {
  background-color: hsla(var(--background), 0.8);
}

/* Explicitly set fixed position and transitions */
#forgestack-dropdown {
  position: fixed; /* Redundant but explicit */
  transition: opacity 0.3s ease, transform 0.3s ease, box-shadow 0.3s ease;
  overflow: hidden; /* Ensure laser effect stays within bounds */
  box-shadow: 0 0 1px 1px rgba(41, 171, 226, 0.1), 0 0 1px 1px rgba(23, 32, 51, 0.5), 0 0 5px 1px rgba(166, 109, 196, 0.1);
}

#forgestack-dropdown:hover {
  box-shadow: 0 0 5px 1px rgba(41, 171, 226, 0.1), 0 0 5px 1px rgba(252, 88, 81, 0.1), 0 0 5px 1px rgba(70, 214, 172, 0.1);
}

/* State handling with direct classes instead of data attributes to prevent flickering */
#forgestack-dropdown[data-state='closed'] {
  opacity: 0;
  transform: translateX(0.5rem);
  pointer-events: none;
}

#forgestack-dropdown[data-state='open'] {
  opacity: 1;
  transform: translateX(0);
  pointer-events: auto;
}

</style>
