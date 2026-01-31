<template>
  <div class="w-full min-h-screen bg-emi-bg-dark text-slate-100 overflow-x-hidden">
    <!-- Mobile-only menu button: Only show if not on /auth page -->
    <div v-if="!isAuthPage" class="md:hidden fixed top-9 left-7 z-[60]">
      <button
        id="mobile-menu-button"
        @click="toggleMobileMenu"
        :class="['w-[3rem] h-[3rem] flex flex-col items-center justify-center gap-[0.5rem]',
                 'bg-emi-primary/20 hover:bg-emi-primary/30',
                 'border border-transparent hover:border-emi-primary/50',
                 'rounded-lg',
                 'shadow-[0_0.25rem_0.75rem_rgba(79,209,197,0.25)] hover:shadow-[0_0.5rem_1.5rem_rgba(79,209,197,0.35)]',
                 'transition-all duration-300 group relative overflow-hidden']"
        :aria-expanded="isMobileMenuOpen"
        aria-haspopup="true"
      >
        <div class="absolute inset-0 bg-gradient-to-r from-emi-primary/0 via-emi-primary/10 to-emi-primary/0 opacity-0 group-hover:opacity-100 transition-opacity duration-500 blur-xl pointer-events-none"></div>
        <span :style="mobileMenuLine1Style" class="block w-5 h-[0.125rem] bg-slate-100 transition-transform duration-300 origin-center"></span>
        <span :style="mobileMenuLine2Style" class="block w-5 h-[0.125rem] bg-slate-100 transition-transform duration-300 origin-center"></span>
        <div class="absolute inset-0 bg-gradient-to-b from-white/0 via-slate-100/5 to-white/0 opacity-0 group-hover:opacity-100 transition-opacity"></div>
        <div class="absolute inset-0 border border-emi-primary/30 rounded-lg opacity-0 group-hover:opacity-100 transition-opacity scale-[1.02]"></div>
      </button>
    </div>

    <!-- Mobile menu overlay - Themed: Only show if not on /auth page and menu is open -->
    <div v-if="!isAuthPage" :class="['mobile-menu-overlay md:hidden fixed inset-0 bg-emi-bg-dark/95 backdrop-blur-md z-50 transition-opacity duration-300', isMobileMenuOpen ? 'opacity-100 pointer-events-auto' : 'opacity-0 pointer-events-none']">
       <nav class="flex flex-col items-center justify-center h-full px-6">
        <div class="py-4 w-full max-w-md">
          <div class="py-3 mb-6 text-center">
            <router-link to="/d" @click="closeMobileMenu" class="inline-block">
              <img src="/IMG/AXLG.svg" alt="Emi App" class="w-[4.9rem] h-auto object-contain opacity-80" />
            </router-link>
          </div>
          
          <div class="space-y-2">
            <router-link @click="closeMobileMenu" to="/d" class="block py-3 px-4 text-slate-100 hover:text-white text-lg text-center rounded-lg hover:bg-emi-primary/20 transition-all relative group overflow-hidden">
              <span class="relative z-10">Dashboard</span>
            </router-link>
            <router-link @click="closeMobileMenu" to="/fs" class="block py-3 px-4 text-slate-100 hover:text-white text-lg text-center rounded-lg hover:bg-emi-primary/20 transition-all relative group overflow-hidden">
              <span class="relative z-10">ForgeStack</span>
            </router-link>
            <router-link @click="closeMobileMenu" to="/emi" class="block py-3 px-4 text-slate-100 hover:text-white text-lg text-center rounded-lg hover:bg-emi-primary/20 transition-all relative group overflow-hidden">
              <span class="relative z-10">Emi Chat</span>
            </router-link>
            <router-link @click="closeMobileMenu" to="/s" class="block py-3 px-4 text-slate-100 hover:text-white text-lg text-center rounded-lg hover:bg-emi-primary/20 transition-all relative group overflow-hidden">
              <span class="relative z-10">Support</span>
            </router-link>
            <router-link @click="closeMobileMenu" to="/a" class="block py-3 px-4 text-slate-100 hover:text-white text-lg text-center rounded-lg hover:bg-emi-primary/20 transition-all relative group overflow-hidden">
              <span class="relative z-10">Account</span>
            </router-link>
          </div>
        </div>
       </nav>
    </div>

    <!-- Main Layout (Sidebar + Content) -->
    <div :class="['flex h-screen overflow-hidden', {'mobile-menu-active': isMobileMenuOpen && !isAuthPage}]">
      <!-- Sidebar: Only show if not on /auth page -->
      <AuthNav
        v-if="!isAuthPage"
        class="w-[20vw] min-w-[20vw] max-w-[20vw] flex-shrink-0 bg-emi-bg-dark scrollbar-none"
        @item-selected-for-input="handleAuthNavItemSelect"
      />

    <!-- Main content wrapper -->
    <div class="flex-1 bg-emi-bg-dark flex flex-col h-screen overflow-hidden relative">
        <div class="absolute inset-0 bg-emi-bg-dark pointer-events-none"></div>
        
        <!-- Content area -->
        <div class="flex-1 bg-emi-bg-dark overflow-hidden pt-6 px-4 md:pr-6 md:pl-0 relative z-10"> 
          <div 
            class="w-full h-full overflow-hidden shadow-xl bg-emi-bg-dark/90 backdrop-blur-xl border border-emi-primary/30"
            :class="{'rounded-tl-[1.875rem] rounded-tr-[1.875rem]': !isAuthPage, 'rounded-lg': isAuthPage}" 
            id="emi-content-container"
          >
            <div class="h-full"> <!-- Removed overflow-auto, overflow-x-hidden -->
              <router-view />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { useRoute } from 'vue-router';
import AuthNav from './AuthNav.vue';
import { useChatStore } from '@/ai/store/chatStore'; // Import chatStore

const route = useRoute();
const isAuthPage = computed(() => route.name === 'Auth');

const isMobileMenuOpen = ref(false);

function handleAuthNavItemSelect(itemText: string) {
  const chatStore = useChatStore();
  chatStore.setSelectedTextForInput(itemText);
  // Optionally, you might want to switch the view in ChatPanel back to 'chat'
  // or close the AuthNav section if it's acting like a modal.
  // For now, just populating the store.
}

function toggleMobileMenu() {
  if (isAuthPage.value) return; 
  isMobileMenuOpen.value = !isMobileMenuOpen.value;
  document.body.classList.toggle('mobile-menu-active', isMobileMenuOpen.value);
}

function closeMobileMenu() {
  isMobileMenuOpen.value = false;
  document.body.classList.remove('mobile-menu-active');
}

const mobileMenuLine1Style = computed(() => ({
  transform: isMobileMenuOpen.value && !isAuthPage.value ? 'rotate(45deg) translateY(0.375rem)' : 'none',
}));

const mobileMenuLine2Style = computed(() => ({
  transform: isMobileMenuOpen.value && !isAuthPage.value ? 'rotate(-45deg) translateY(-0.375rem)' : 'none',
}));

onMounted(() => {
  // Logic specific to AuthLayout on mount
});

onUnmounted(() => {
  document.body.classList.remove('mobile-menu-active'); 
});

</script>

<style scoped>
/* Scoped styles specific to AuthLayout.vue */
:global(body.mobile-menu-active) {
  overflow: hidden;
}
</style>
