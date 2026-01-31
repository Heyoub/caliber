<script setup lang="ts">
import { ref, computed } from 'vue';
import { useRouter } from 'vue-router';
import { Menu, ChevronRight, FileText, Briefcase, Archive, Settings, LogOut } from 'lucide-vue-next';
import { useProjectStore } from '@/ai/store/projectStore'; // Corrected path
import { useAuth } from '@/ai/composables/useAuth'; // Corrected path

defineProps<{ isOpen: boolean }>();
const emit = defineEmits<{
  (e: 'toggle'): void;
  (e: 'selectCase', id: string): void;
  (e: 'selectProject', id: string): void;
  (e: 'selectChat', id: string): void; // Assuming chat selection might be needed
  (e: 'newChat'): void;
  (e: 'openSettings'): void; // For opening a settings modal/page
}>();

const projectStore = useProjectStore();
const router = useRouter();
const { signOut: logout, isAuthenticated } = useAuth(); // Changed logout to signOut and aliased

const sectionOpenState = ref({ chats: true, cases: true, projects: true });
const toggleSection = (key: 'chats' | 'cases' | 'projects') => {
  sectionOpenState.value[key] = !sectionOpenState.value[key];
};

// TODO: Replace with actual chat session data, possibly from chatStore
// Provide a type for the computed property if it's expected to hold Chat items
import type { Chat } from '@/ai/types/chat.types'; // Assuming Chat type is exported
const chats = computed<Chat[]>(()=> [
  // Example chat data
  // { id: 'chat1', title: 'Recent Chat 1', messages: [], createdAt: new Date().toISOString() },
  // { id: 'chat2', title: 'Important Discussion', messages: [], createdAt: new Date().toISOString() },
]);

const handleLogout = async () => {
  try {
    await logout();
    router.push('/'); // Redirect to home or login page after logout
  } catch (error) {
    console.error("Error during logout:", error);
    // Handle logout error (e.g., show a notification)
  }
};

const navigateToKnowledgeBase = () => {
  router.push({ name: 'KnowledgeBase' }); // Use named route
  emit('toggle'); // Close sidebar on navigation
};

const navigateToProjects = () => {
  router.push({ name: 'Projects' }); // Use named route
  emit('toggle'); // Close sidebar on navigation
};

</script>

<template>
  <!-- Sidebar Toggle Button (fixed position) -->
  <button @click="$emit('toggle')"
          class="fixed top-4 left-4 z-50 h-10 w-10 rounded-full flex items-center
                 justify-center bg-slate-800/50 hover:bg-slate-700/80 text-slate-200 transition">
    <Menu class="h-5 w-5" />
  </button>

  <aside :class="['fixed top-0 left-0 h-screen w-72 bg-slate-900 border-r border-slate-800 border-solid shadow-2xl',
                    isOpen ? 'translate-x-0' : '-translate-x-full',
                    'transition-transform duration-300 ease-in-out z-40 flex flex-col']">
    
    <!-- Header / Logo Area -->
    <div class="h-[3.75rem] flex items-center justify-center border-b border-slate-800 border-solid">
      <!-- Replace with your logo component or text -->
      <span class="text-xl font-semibold text-slate-100">Council AI</span>
    </div>

    <nav class="flex-1 overflow-y-auto p-2 space-y-1">
      <!-- Chats -->
      <section class="py-1">
        <header @click="toggleSection('chats')"
                class="flex items-center gap-2 p-2 rounded hover:bg-slate-800 cursor-pointer text-slate-300 hover:text-slate-100">
          <FileText class="h-4 w-4 flex-shrink-0" />
          <span class="flex-1 text-sm font-medium">Chats</span>
          <ChevronRight :class="sectionOpenState.chats ? 'rotate-90' : ''" class="h-4 w-4 transition-transform" />
        </header>
        <div v-if="sectionOpenState.chats" class="pl-6 mt-1 space-y-0.5">
          <button v-for="c in chats" :key="c.id"
                  class="block w-full text-left text-xs py-1.5 px-2 rounded hover:bg-slate-700/70 text-slate-400 hover:text-slate-200 transition-colors"
                  @click="$emit('selectChat', c.id)">
            {{ c.title }} <!-- Changed from c.identifier.displayTitle to c.title -->
          </button>
          <button @click="$emit('newChat')" class="block w-full text-left text-xs py-1.5 px-2 rounded text-sky-400 hover:bg-sky-400/10 hover:text-sky-300 transition-colors">
            + New Chat
          </button>
        </div>
      </section>

      <!-- Cases -->
      <section class="py-1">
        <header @click="toggleSection('cases')" class="flex items-center gap-2 p-2 rounded hover:bg-slate-800 cursor-pointer text-slate-300 hover:text-slate-100">
          <Briefcase class="h-4 w-4 flex-shrink-0" />
          <span class="flex-1 text-sm font-medium">Cases</span>
          <ChevronRight :class="sectionOpenState.cases ? 'rotate-90' : ''" class="h-4 w-4 transition-transform" />
        </header>
        <div v-if="sectionOpenState.cases" class="pl-6 mt-1 space-y-0.5">
          <button v-for="c in projectStore.cases" :key="c.id"
                  @click="router.push({ name: 'ItemView', params: { type: 'case', id: c.id } }); emit('toggle'); $emit('selectCase', c.id)"
                  class="block w-full text-left text-xs py-1.5 px-2 rounded hover:bg-slate-700/70 text-slate-400 hover:text-slate-200 transition-colors">
            {{ c.name }}
          </button>
          <!-- Add New Case Button if applicable -->
        </div>
      </section>

      <!-- Projects -->
      <section class="py-1">
        <header @click="toggleSection('projects')" class="flex items-center gap-2 p-2 rounded hover:bg-slate-800 cursor-pointer text-slate-300 hover:text-slate-100">
          <Archive class="h-4 w-4 flex-shrink-0" />
          <span class="flex-1 text-sm font-medium">Projects</span>
          <ChevronRight :class="sectionOpenState.projects ? 'rotate-90' : ''" class="h-4 w-4 transition-transform" />
        </header>
        <div v-if="sectionOpenState.projects" class="pl-6 mt-1 space-y-0.5">
          <button v-for="p in projectStore.projects" :key="p.id"
                  @click="router.push({ name: 'ItemView', params: { type: 'project', id: p.id } }); emit('toggle'); $emit('selectProject', p.id)"
                  class="block w-full text-left text-xs py-1.5 px-2 rounded hover:bg-slate-700/70 text-slate-400 hover:text-slate-200 transition-colors">
            {{ p.name }}
          </button>
          <!-- Add New Project Button if applicable -->
        </div>
      </section>

       <!-- Knowledge Base Link -->
      <section class="py-1">
        <button @click="navigateToKnowledgeBase" class="flex items-center gap-2 p-2 rounded hover:bg-slate-800 cursor-pointer text-slate-300 hover:text-slate-100 w-full">
          <!-- Use an appropriate icon for Knowledge Base, e.g., BookOpen -->
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-book-open flex-shrink-0"><path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/></svg>
          <span class="flex-1 text-sm font-medium text-left">Knowledge Base</span>
        </button>
      </section>

    </nav>

    <!-- Footer / Settings / Logout -->
    <div class="p-2 border-t border-slate-800 border-solid mt-auto">
      <button @click="$emit('openSettings')" class="flex items-center gap-2 w-full p-2 rounded hover:bg-slate-800 text-slate-300 hover:text-slate-100 transition-colors">
        <Settings class="h-4 w-4 flex-shrink-0" />
        <span class="text-sm font-medium">Settings</span>
      </button>
      <button v-if="isAuthenticated" @click="handleLogout" class="flex items-center gap-2 w-full p-2 mt-1 rounded hover:bg-red-500/20 text-red-400 hover:text-red-300 transition-colors">
        <LogOut class="h-4 w-4 flex-shrink-0" />
        <span class="text-sm font-medium">Logout</span>
      </button>
    </div>
  </aside>
</template>

<style scoped>
/* Basic styling, Tailwind handles most of it */
aside {
  /* You can add custom scrollbar styling here if needed for the nav */
}
.transition-transform {
  transition-property: transform;
}
</style>