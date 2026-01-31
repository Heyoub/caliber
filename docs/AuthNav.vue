<template>
  <YtmndNavLink />
<nav class="flex flex-col bg-emi-bg-dark min-h-screen px-2 pt-[1.65rem] relative z-50 text-slate-100"> <!-- Use emi-bg-dark -->
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
        <span class="h-8 w-3 flex flex-col ml-4 mt-[1rem] items-center justify-center gap-[0.5rem]">
          <span ref="line1Ref" class="w-[1.875rem] h-[0.33rem] bg-slate-100/90 rounded-full transition-all duration-300 origin-center" data-menu-line="forgestack-1"></span>
          <span ref="line2Ref" class="w-[1.875rem] h-[0.33rem] bg-slate-100/90 rounded-full transition-all duration-300 origin-center" data-menu-line="forgestack-2"></span>
        </span>
        <!-- Logo Image (replaces text) -->
        <img src="/IMG/AXLG.svg" alt="ForgeStack Logo" class="h-14 w-auto opacity-90" />
        <span class="mt-[1rem] ml-[-1.75rem] text-3xl font-semibold text-white tracking-wider">orgeStack</span>
      </span>
    </GlowPressButton>

    <!-- ForgeStack Dropdown Menu - Themed for glassmorphism -->
    <div
      id="forgestack-dropdown"
      ref="dropdownRef"
      class="absolute mt-3 w-[10rem] h-[auto] bg-[#1a2e35e6] backdrop-blur-xl border border-[#4fd1c54d] border-solid rounded-lg shadow-[0_0.25rem_0.25rem_rgba(79,209,197,0.15)] overflow-hidden opacity-0 translate-y-2 pointer-events-none transition-all duration-300"
      data-dropdown="forgestack"
      :data-state="isDropdownOpen ? 'open' : 'closed'"
      :style="floatingStyles"
    >
      <!-- Laser effect layer - can use primary color -->
      <div ref="laserEffectRef" id="forgestack-laser-effect" class="absolute inset-0 pointer-events-none opacity-10 mix-blend-plus-lighter transform-gpu hover:opacity-20 transition-opacity duration-500"></div>

      <!-- Links -->
      <div class="relative z-10 py-1">
        <div v-for="(parentLink, index) in forgestackLinks" :key="parentLink.text" class="category-block">
          <hr v-if="index > 0" class="border-t border-[#4fd1c54d] border-solid my-1 mx-4" />

          <div v-if="parentLink.children && parentLink.children.length > 0" class="flex items-center gap-2 px-[1rem] pt-[0.75rem] pb-[0.25rem] text-xs font-semibold text-slate-300/80 uppercase tracking-wider">
            <component :is="parentLink.icon" class="w-4 h-4 text-[#4FD1C5] opacity-80" /> <!-- Icon with primary color -->
            <span>{{ parentLink.text }}</span>
          </div>

          <template v-if="parentLink.children">
            <router-link
              v-for="childLink in parentLink.children"
              :key="childLink.href"
              :to="childLink.href"
              
              class="block px-[1rem] py-[0.5rem] text-slate-100/90 hover:text-white text-left
                     hover:bg-[#4FD1C5]/10 transition-all duration-300
                     relative group/link text-sm"
              @mousemove="handleMouseMove"
              :aria-current="currentPath === childLink.href ? 'page' : undefined"
            >
              <span class="relative z-10">{{ childLink.text }}</span>
              <span class="absolute bottom-0 left-1/2 right-1/2 h-px bg-[#4FD1C5]/80
                         group-hover/link:left-[1rem] group-hover/link:right-[1rem]
                         transition-all duration-300
                         shadow-[0_0.5rem_1.5rem_rgba(79,209,197,0.5)]
                         after:absolute after:inset-0 after:bg-[#4FD1C5]/20 after:blur-sm after:opacity-0 after:group-hover/link:opacity-100 after:transition-opacity">
              </span>
            </router-link>
          </template>
        </div>
      </div>
    </div>
    
    <!-- This div is the "middle content area" that will host the current page's specific tabs -->
    <div class="flex-grow flex flex-col overflow-hidden mt-4 mb-4 border border-emi-primary/30 rounded-lg bg-emi-bg-dark">
      <!-- Conditionally render the correct tab component based on the route -->
      <EmiTabs v-if="currentPath === '/emi'" @item-selected-for-input="handleItemSelectedForInput" />
      <SettingsTabs v-else-if="currentPath === '/a'" />
      <SupportTabs v-else-if="currentPath === '/s'" />
      <!-- Optional: A placeholder if no specific tabs for the current route -->
      <div v-else class="p-4 text-sm text-slate-400 text-center h-full flex items-center justify-center">
        <!-- No specific navigation items for this page. -->
      </div>
    </div>

      <!-- Footer-like Info Section - Themed -->
      <div class="mt-auto pt-4 px-2 pb-4 text-center">
        <div class="px-2 py-3 rounded-lg bg-slate-800/50 backdrop-blur-sm border border-[#4fd1c54d] border-solid">
          <h3 class="text-xs font-semibold mb-2 text-[#4FD1C5] uppercase tracking-wider">
            Hi, {{ user.name }}
          </h3>
          <div class="text-xs text-slate-300/70 space-y-1">
            <div class="flex justify-center gap-3">
              <router-link to="/s" class="hover:text-[#4FD1C5] transition-colors">Support</router-link>
              <span>&bull;</span>
              <a href="/terms" target="_blank" class="hover:text-[#4FD1C5] transition-colors">Terms</a>
            </div>
          </div>
        </div>
    </div>
  </nav>
</template>

<script setup lang="ts">
import YtmndNavLink from '@/components/common/YtmndNavLink.vue';
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue';
import { useFloating, offset, flip, shift, autoUpdate } from '@floating-ui/vue';
import { useRoute } from 'vue-router';
import GlowPressButton from '../ui/GlowPressButton.vue';
import { LogOut, LayoutPanelLeft } from 'lucide-vue-next';
// PromptLibrary, TemplateLibrary, etc. are now imported within specific Tab components (e.g., EmiTabs)
// import PromptLibrary from '@/ai/components/common/PromptLibrary.vue';
// import TemplateLibrary from '@/ai/components/common/TemplateLibrary.vue';
// import CouncilReasoningPanel from '@/ai/components/common/CouncilReasoningPanel.vue';
// import ChatHistory from '@/ai/components/common/ChatHistory.vue';
// import DocumentUploader from '@/ai/components/common/DocumentUploader.vue';
import { useAiContextStore } from '@/ai/store/aiContextStore'; // ViewMode might not be needed here anymore
import DashTabs from './Navigation/DashTabs.vue'; // Corrected path
import EmiTabs from './Navigation/EmiTabs.vue';   // Corrected path
import SettingsTabs from './Navigation/SettingsTabs.vue'; // Corrected path
import SupportTabs from './Navigation/SupportTabs.vue'; // Corrected path


// --- Props / Route ---
const route = useRoute();
const currentPath = computed(() => route.path);

// --- Refs --- 
const buttonRef = ref<InstanceType<typeof GlowPressButton> | null>(null);
const dropdownRef = ref<HTMLElement | null>(null);
const laserEffectRef = ref<HTMLElement | null>(null);
const line1Ref = ref<HTMLElement | null>(null); // Ref for hamburger line 1
const line2Ref = ref<HTMLElement | null>(null); // Ref for hamburger line 2
const user = ref({
  name: 'John Doe',
});

// --- Floating UI Setup ---
const { floatingStyles, update } = useFloating(buttonRef, dropdownRef, {
  placement: 'right-start', // Adjust placement as needed
  middleware: [
    offset(25), // Increased offset from 25 to 30 for a "smidge" more space
    flip(),    // Flip placement if it overflows
    shift({ padding: 25 }) // Shift to stay in view, with 25px padding from edges
  ],
  whileElementsMounted: autoUpdate, // Keep position updated automatically
});

// --- State --- 
// Use null to indicate no menu is active, mirroring the original script's logic
const activeMenu = ref<'forgestack' | null>(null);
const isDropdownOpen = computed(() => activeMenu.value === 'forgestack');
// activeAuthNavTab and activeLibraryView are removed as this logic moves to individual Tab components
const aiContextStore = useAiContextStore();

// --- Data ---
const forgestackLinks = [
  { 
    text: 'Menu',
    href: '#',
    icon: LayoutPanelLeft, // Changed Chip to LayoutPanelLeft (example)
    children: [
      { text: 'Emi', href: '/emi'},
      { text: 'Support', href: '/s'},
      { text: 'Account', href: '/a' }
    ]
  },
  {
    text: 'Log-out',
    href: '#',
    icon: LogOut, // Using lock icon for login
    children: [
      { text: 'Log-out', href: '/l' }
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

const emitAuthNavActions = defineEmits(['item-selected-for-input']);

function handleItemSelectedForInput(itemText: string) {
  emitAuthNavActions('item-selected-for-input', itemText);
  // When an item is selected from library/template, switch main view to chat
  aiContextStore.setViewMode('chat');
}

// setAuthNavTab function is removed as tab logic is now within individual Tab components.

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
/* ... existing styles ... */

/* Explicitly set fixed position and transitions for dropdown */
#forgestack-dropdown {
  position: fixed;
  transition: opacity 0.3s ease, transform 0.3s ease, box-shadow 0.3s ease;
  overflow: hidden;
  box-shadow: 0 0 1px 1px rgba(41, 171, 226, 0.1), 0 0 1px 1px rgba(23, 32, 51, 0.5), 0 0 5px 1px rgba(166, 109, 196, 0.1);
}

#forgestack-dropdown:hover {
  box-shadow: 0 0 5px 1px rgba(41, 171, 226, 0.1), 0 0 5px 1px rgba(252, 88, 81, 0.1), 0 0 5px 1px rgba(70, 214, 172, 0.1);
}

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

/* Hide scrollbar for the AuthNav content area */
.authnav-content-scrollable::-webkit-scrollbar {
  display: none; /* For Chrome, Safari, and Opera */
}
.authnav-content-scrollable {
  -ms-overflow-style: none;  /* For Internet Explorer and Edge */
  scrollbar-width: none;  /* For Firefox */
}
</style>
