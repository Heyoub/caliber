<template>
    <!-- System message (centered, italic text) -->
    <div v-if="message.role === 'system'" class="text-center text-sm italic text-gray-400 my-2">
      {{ message.content }}
    </div>
  
    <!-- User or Assistant message bubble -->
    <div
      v-else
      :class="[
        'bubble',
        'my-1 rounded-lg' // Base container style from ChatMessage.tsx
      ]"
    >
      <div class="flex items-start p-3" :class="[message.role === 'user' ? 'justify-end' : '']">
        <!-- Avatar -->
        <div :class="['flex-shrink-0', message.role === 'user' ? 'order-last ml-2' : 'mr-2']">
          <div class="w-6 h-6 rounded-full flex items-center justify-center bg-background/80 dark:bg-background/60 shadow-sm">
            <User v-if="message.role === 'user'" class="w-3 h-3 text-orange-500 dark:text-orange-400" />
            <Bot v-else class="w-3 h-3 text-purple-500 dark:text-purple-400" />
          </div>
        </div>

        <!-- Message Content Area -->
        <div
          :class="[
                                                    'flex-1 min-w-0 overflow-hidden max-w-full p-3 rounded-xl shadow-md', // Base padding and rounding for content
                                                    // Consistent glassmorphic style for both user and assistant
                                                    'bg-slate-700/20 backdrop-blur-lg border border-slate-500/30 border-solid border-solid', // Dark glassmorphic
                                                    'text-slate-100 dark:text-slate-50' // Ensure good text contrast
                                                  ]"
       >
         <!-- User/Assistant Label - C10 Redact User Name -->
         <div class="text-xs font-medium mb-1.5"
           :class="message.role === 'user' ? 'text-orange-400 dark:text-orange-300' : 'text-purple-400 dark:text-purple-300'">
           {{ message.role === 'user' ? 'User' : (message.model || 'Assistant') }}
         </div>
         
         <!-- Assistant model name metadata (if available and not shown as main label) -->
         <div v-if="message.role === 'assistant' && message.model && (message.model !== (message.model || 'Assistant'))" class="text-[0.625rem] text-slate-400 dark:text-slate-400 opacity-70 mb-1">
             {{ message.model }}
          </div>

          <!-- Typing indicator (Sparkles + dot-flashing) -->
      <div v-if="message.typing" class="flex p-1 items-center gap-3">
         <div class="w-6 h-6 flex-shrink-0 rounded-full bg-gradient-to-br from-purple-500/30 to-orange-500/20 flex items-center justify-center">
           <Sparkles class="w-3 h-3 text-purple-400 dark:text-purple-300" />
         </div>
         <div class="dot-flashing"></div>
      </div>

          <!-- Message content (Markdown rendered to HTML for assistant, plain text for user) -->
          <!-- Added break-words whitespace-pre-wrap for robust wrapping -->
          <div v-else class="text-sm break-words whitespace-pre-wrap" v-html="renderedContent"></div>
        </div>
        <!-- Copy to Clipboard Button (top right of bubble) -->
        <button
          
          @click.stop="handleCopy"
          :title="isCopying ? 'Copying...' : 'Copy to clipboard'"
          class="absolute top-2 right-2 p-1 rounded hover:bg-slate-200/60 dark:hover:bg-slate-700/60 transition"
          style="z-index:2"
        >
          <Clipboard class="w-4 h-4" :class="isCopying ? 'text-teal-500 animate-pulse' : 'text-gray-400'" />
        </button>
      </div>
    </div>
  </template>

  <script setup lang="ts">
  import { computed, ref } from 'vue';
  import { Sparkles, User, Bot, Clipboard } from 'lucide-vue-next';
  import { marked } from 'marked';
  import { useClipboard } from '@/ai/composables/useClipboard'; // Corrected path
  import type { ChatMessage } from '@/ai/types/chat.types'; // Import the shared ChatMessage type
  
  // Use the imported ChatMessage type for props
  const props = defineProps<{ message: ChatMessage }>();
  
  // Compute rendered HTML content for the message.
  // If it's assistant content, parse Markdown to HTML; for others, escape as needed.
  const renderedContent = computed(() => {
    if (props.message.role === 'assistant') {
      // Convert Markdown to HTML (assumes content is safe or sanitized separately)
      return marked.parse(props.message.content);
    }
    // For user or system messages (which are plain text), simply escape HTML.
    // marked.parse can handle plain text too, but we ensure no script injection for user content.
    return props.message.content
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
  });
  const { copyToClipboard, isCopying } = useClipboard();
  
  function handleCopy() {
    copyToClipboard(props.message.content); // Changed copy to copyToClipboard
  }
  </script>
  
  <style scoped>
  .bubble {
    position: relative;
  }
  /* Base bubble styling - removed as structure changed */
  
  /* Typing indicator - dot-flashing animation from AIChat.tsx */
  .dot-flashing {
    position: relative;
    width: 6px;
    height: 6px;
    border-radius: 5px;
    background-color: #9880ff; /* Example color, adjust as needed */
    color: #9880ff;
    animation: dot-flashing 1s infinite linear alternate;
    animation-delay: 0.5s;
  }
  .dot-flashing::before,
  .dot-flashing::after {
    content: '';
    display: inline-block;
    position: absolute;
    top: 0;
    width: 6px;
    height: 6px;
    border-radius: 5px;
    background-color: #9880ff;
    color: #9880ff;
  }
  .dot-flashing::before {
    left: -10px;
    animation: dot-flashing 1s infinite alternate;
    animation-delay: 0s;
  }
  .dot-flashing::after {
    left: 10px;
    animation: dot-flashing 1s infinite alternate;
    animation-delay: 1s;
  }

  @keyframes dot-flashing {
    0% { background-color: #9880ff; }
    50%, 100% { background-color: rgba(152, 128, 255, 0.2); } /* Adjust opacity/color for flashed state */
  }

   /* Basic markdown content styling within bubbles */
  .bubble h1, .bubble h2, .bubble h3, .bubble h4, .bubble h5, .bubble h6 {
    @apply font-grotesk font-semibold;
  }
  .bubble p {
    margin: 0.5rem 0;
  }
  .bubble code {
    background-color: rgba(255,255,255,0.1);
    padding: 0.1rem 0.4rem;
    border-radius: 0.25rem;
    font-family: monospace;
  }
  .bubble pre {
    background-color: rgba(255,255,255,0.05);
    padding: 0.5rem;
    border-radius: 0.25rem;
    overflow-x: auto;
  }
  </style>
  