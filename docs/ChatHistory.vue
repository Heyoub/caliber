<template>
  <div class="chat-history p-4 text-slate-200 h-full flex flex-col bg-slate-800/30 rounded-lg">
    <div class="flex justify-between items-center mb-4 flex-shrink-0">
      <h2 class="text-xl font-semibold text-slate-100">Chat History</h2>
      <button
        @click="startNewChat"
        class="px-3 py-1.5 text-xs font-medium rounded-md transition-colors duration-150 bg-teal-600 hover:bg-teal-500 text-white"
        title="Start a new chat"
      >
        <PlusCircle class="w-4 h-4 inline-block mr-1 -mt-0.5" /> New Chat
      </button>
    </div>

    <div v-if="sortedChats.length === 0" class="text-center text-slate-400 flex-grow flex items-center justify-center">
      <p>No chat history yet. Start a new chat!</p>
    </div>

    <div v-else class="flex-grow overflow-y-auto custom-scrollbar-vertical-emi pr-1 space-y-2">
      <div
        v-for="chat in sortedChats"
        :key="chat.id"
        @click="selectChat(chat.id)"
        :class="[
          'p-3 rounded-lg cursor-pointer transition-all duration-150 group',
          chat.id === activeChatId ? 'bg-teal-600/30 ring-1 ring-teal-500' : 'bg-slate-700/40 hover:bg-slate-600/60'
        ]"
      >
        <div class="flex justify-between items-center">
          <p class="font-medium text-sm truncate text-slate-100 group-hover:text-teal-300">
            {{ chat.title || 'Untitled Chat' }}
          </p>
          <button
            @click.stop="confirmDeleteChat(chat.id)"
            class="p-1 rounded text-slate-500 hover:text-red-400 opacity-50 group-hover:opacity-100 transition-opacity"
            title="Delete chat"
          >
            <Trash2 class="w-4 h-4" />
          </button>
        </div>
        <p class="text-xs text-slate-400 mt-1">
          {{ formatDate(chat.createdAt) }} - {{ chat.messages.length }} message(s)
        </p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { storeToRefs } from 'pinia';
import { useChatStore } from '@/ai/store/chatStore';
import { useAiContextStore } from '@/ai/store/aiContextStore';
import { PlusCircle, Trash2 } from 'lucide-vue-next';

const chatStore = useChatStore();
const aiContextStore = useAiContextStore();

const { chats, activeChatId } = storeToRefs(chatStore);

const sortedChats = computed(() => {
  // Sort chats by creation date, newest first
  return [...chats.value].sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime());
});

const selectChat = (chatId: string) => {
  chatStore.setActiveChatId(chatId);
  aiContextStore.setViewMode('chat'); // Switch to chat view when a history item is selected
};

const startNewChat = () => {
  chatStore.addChat(); // This will also set the new chat as active
  aiContextStore.setViewMode('chat');
};

const confirmDeleteChat = (chatId: string) => {
  // Using browser's confirm dialog for simplicity.
  // A custom modal would be better for production UX.
  const chatToDelete = chats.value.find(c => c.id === chatId);
  if (window.confirm(`Are you sure you want to delete the chat "${chatToDelete?.title || 'this chat'}"? This cannot be undone.`)) {
    chatStore.deleteChat(chatId);
  }
};

const formatDate = (isoString: string) => {
  if (!isoString) return 'Unknown date';
  try {
    return new Date(isoString).toLocaleDateString(undefined, {
      year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit'
    });
  } catch (e) {
    return 'Invalid date';
  }
};

// TODO: Implement fetchChatSessions action in chatStore and call it onMounted
// onMounted(async () => {
//   await chatStore.fetchChatSessions();
// });

</script>

<style scoped>
/* Styles for ChatHistory.vue */
/* Assuming custom-scrollbar-vertical-emi might be defined globally or in a parent.
   If specific styling for this component's scrollbar is needed, it can be refined here.
   The class 'custom-scrollbar-vertical-emi' is applied to the scrollable div.
   The styles below are a general version of that scrollbar theme.
*/
.custom-scrollbar-vertical-emi::-webkit-scrollbar {
  width: 6px; /* Slightly thinner */
}
.custom-scrollbar-vertical-emi::-webkit-scrollbar-track {
  background: transparent;
  border-radius: 3px;
}
.custom-scrollbar-vertical-emi::-webkit-scrollbar-thumb {
  background-color: theme('colors.slate.600 / 0.5'); /* Adjusted for general theme */
  border-radius: 3px;
}
.custom-scrollbar-vertical-emi::-webkit-scrollbar-thumb:hover {
  background-color: theme('colors.teal.500 / 0.6'); /* Adjusted for general theme */
}
.custom-scrollbar-vertical-emi { /* Fallback for Firefox */
  scrollbar-width: thin;
  scrollbar-color: theme('colors.slate.600 / 0.5') transparent; /* Adjusted */
}
</style>