<template>
    <!-- Main chat panel container -->
    <div class="relative flex flex-col h-full min-h-[97.5vh] bg-surface-dark/0 rounded-t-[0rem]">
      <!-- Gradient Background Layer -->
      <div class="absolute inset-0 bg-gradient-to-br from-[#a855f7]/30 via-[#06b6d4]/20 to-[#f59e42]/30 dark:from-[#a855f7]/50 dark:via-[#06b6d4]/30 dark:to-[#f59e42]/50 blur-2xl opacity-80 rounded-t-[0rem] z-0"></div>
      <!-- Add the Chat Header, passing current view and setter -->
      <ChatHeader
        :is-sending="isSending"
        :current-view="viewMode"
        :is-authenticated="isAuthenticated"
        :theme="props.theme"
        @set-view="setViewMode"
        @quick-prompt-selected="handleQuickPromptSelectedFromHeader"
        @quick-template-selected="handleQuickTemplateSelectedFromHeader"
        class="relative z-10"
      />

      <!-- Document Uploader Modal/Overlay -->
      <div v-if="showDocumentUploader" class="absolute inset-0 z-30 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
        <DocumentUploader
          :theme="props.theme"
          @close="showDocumentUploader = false"
          @files-uploaded="handleFilesUploaded"
        />
      </div>

      <!-- Drag-and-drop overlay container (covers area below header) -->
      <div class="relative flex-1 flex flex-col overflow-hidden z-10 rounded-b-[0rem]"
           @dragenter="onDragEnter"
           @dragleave="onDragLeave"
           @dragover.prevent
           @drop.prevent="onFileDrop">
      <!-- Drag-and-drop overlay (shown when dragging files over the panel) -->
      <div v-if="dragActive" class="absolute inset-0 flex flex-col items-center justify-center bg-black/60 border-2 border-dashed border-teal-400 rounded-lg pointer-events-none z-10">
        <UploadCloud class="w-12 h-12 text-teal-200 mb-3" /> {/* Changed icon and size */}
        <span class="text-teal-100 font-grotesk font-medium text-lg">Drop files to upload</span> {/* Updated text */}
      </div>

      <!-- Main Content Area - Switches based on viewMode OR always chat if theme is 'emi' -->
      <div class="flex-1 overflow-y-auto">
        <!-- Chat View -->
        <!-- Always show chat view if theme is 'emi', otherwise respect viewMode -->
        <div v-if="props.theme === 'emi' || viewMode === 'chat'" class="p-4 h-full flex flex-col">
           <!-- PromptButtons are now in ChatHeader, so removed from here -->
           <!-- <div v-if="isChatEmpty && props.theme === 'default'" class="mb-4 p-3 rounded-xl">
             <PromptButtons @prompt-selected="sendMessage" :theme="props.theme" />
           </div> -->
           <!-- Pass messages from the composable -->
           <ChatMessageList :messages="messages" class="flex-1" />
           <!-- Add Contextual Suggestions below message list (Only for default theme) -->
           <ContextualSuggestions v-if="props.theme === 'default'" @suggestion-selected="sendMessage" class="mt-2 flex-shrink-0" />
        </div>

        <!-- Prompts View (Only for default theme) -->
        <div v-else-if="props.theme === 'default' && viewMode === 'prompts'" class="p-4">
          <h2 class="text-lg font-semibold mb-2 text-white">Prompt Library</h2>
          <p class="text-gray-400">Prompt library component will go here.</p>
          <PromptLibrary @prompt-selected="handlePromptSelected" :theme="props.theme" />
        </div>

        <!-- Templates View (Placeholder) (Only for default theme) -->
        <div v-else-if="props.theme === 'default' && viewMode === 'templates'" class="p-4">
           <h2 class="text-lg font-semibold mb-2 text-white">Templates</h2>
           <TemplateLibrary @template-selected="handleTemplateSelected" :theme="props.theme" />
        </div>

        <!-- Reasoning View (Only for default theme) -->
        <div v-else-if="props.theme === 'default' && viewMode === 'reasoning'" class="p-4">
           <CouncilReasoningPanel />
        </div>

        <!-- History View (Placeholder) (Only for default theme) -->
        <div v-else-if="props.theme === 'default' && viewMode === 'history'" class="p-4">
           <h2 class="text-lg font-semibold mb-2 text-white">Chat History</h2>
           <ChatHistory />
        </div>
        
        <!-- Document Analysis View (Only for default theme) -->
        <div v-else-if="props.theme === 'default' && viewMode === 'documents'" class="overflow-y-auto" style="height: 100%;">
          <DocumentAnalysisDemo />
        </div>
      </div>

      <!-- Input toolbar (Only show if theme is 'emi' OR if theme is 'default' AND viewMode is 'chat') -->
      <!-- Listen for events and call composable functions -->
      <!-- Pass clearChat function as a prop -->
      <ChatInput
        @send="sendMessage"
        @files="handleFiles"
        @mic="handleMic"
        @enhance-prompt="handleEnhancePrompt"
        :clear-chat="clearChat"
        :is-authenticated="isAuthenticated"
        :theme="props.theme"
        @request-signup="$emit('request-signup')"
        @open-document-uploader="showDocumentUploader = true"
        v-if="props.theme === 'emi' || viewMode === 'chat'"
      />
      <!-- RedactionShield is now inside ChatInput.vue -->

      <!-- Optional: Loading overlay while sending -->
      <div v-if="isSending" class="absolute inset-0 flex items-center justify-center bg-black/30 backdrop-blur-sm z-10">
          <span class="text-white font-medium">Processing...</span>
          <!-- Or use a spinner component -->
      </div>
      </div> <!-- Close the drag-and-drop container -->
    </div>
  </template>

  <script setup lang="ts">
  import { ref, computed, type PropType } from 'vue';
  import { storeToRefs } from 'pinia';
  import { provideRedactionStatus } from '@/ai/composables/useRedactionStatus';
  import { asUnredacted } from '@/ai/types/common.types';
  import { useChatStore } from '@/ai/store/chatStore'; // Import chatStore
  import ChatMessageList from './ChatMessageList.vue';
  import ChatInput from './ChatInput.vue';
  import PromptButtons from './PromptButtons.vue';
  import ChatHeader from './ChatHeader.vue';
  // Import placeholder components for other views
  import PromptLibrary from './PromptLibrary.vue';
  import TemplateLibrary from './TemplateLibrary.vue';
  import CouncilReasoningPanel from './CouncilReasoningPanel.vue';
  import ContextualSuggestions from './ContextualSuggestions.vue';
  import ChatHistory from './ChatHistory.vue';
  // DocumentAnalysisDemo might be replaced or integrated with DocumentUploader
  import DocumentAnalysisDemo from './DocumentAnalysisDemo.vue';
  import DocumentUploader from './DocumentUploader.vue'; // Import DocumentUploader
  import { UploadCloud } from 'lucide-vue-next'; // Paperclip no longer needed here
  // Import the composable
  import { useChatLogic } from '@/ai/composables/useChatLogic'; // Corrected path
  import { useContextualSuggestionsGenerator } from '@/ai/composables/useContextualSuggestionsGenerator'; // Added
  // Import AI Context Store for view mode
  import { useAiContextStore } from '@/ai/store/aiContextStore';

  const props = defineProps({
    theme: {
      type: String as PropType<'default' | 'emi'>,
      default: 'default'
    }
  });

  // Use the composable to get state and methods
  const {
    messages,
    isSending,
    sendMessage,
    handleFiles, // Get file handler from composable
    handleMic,   // Get mic handler from composable
    clearChat,
    isAuthenticated // Get isAuthenticated from useChatLogic
  } = useChatLogic();

  // Initialize the contextual suggestions generator
  useContextualSuggestionsGenerator();

  const emit = defineEmits(['request-signup']); // Define the event

  // Provide redaction status for ChatInput and RedactionShield
  provideRedactionStatus();
 
  // Get view mode state and setter from AI Context Store
  const aiContextStore = useAiContextStore();
  const { viewMode } = storeToRefs(aiContextStore); // Make viewMode reactive
  const { setViewMode } = aiContextStore; // Get the action

  // Computed property to check if messages array is empty
  const isChatEmpty = computed(() => !messages.value || messages.value.length === 0);

  // Local state for drag-and-drop UI remains
  const dragActive = ref(false);
  let dragCounter = 0; // Counter to handle nested elements during drag
  const showDocumentUploader = ref(false); // State to control DocumentUploader visibility

  // Drag-and-drop event handlers for file upload overlay
  function onDragEnter(event: DragEvent) {
    // Check if files are being dragged
    if (event.dataTransfer?.types.includes('Files')) {
        dragCounter++;
        dragActive.value = true;
    }
  }
  function onDragLeave() {
    dragCounter--;
    if (dragCounter <= 0) {
        dragActive.value = false;
        dragCounter = 0; // Reset counter
    }
  }
  function onFileDrop(event: DragEvent) {
    dragCounter = 0; // Reset counter
    dragActive.value = false;
    const fileList = event.dataTransfer?.files;
    if (fileList && fileList.length > 0) {
      // Call the composable's file handler - this might just store them
      handleFiles(Array.from(fileList));
      // Then show the uploader UI
      showDocumentUploader.value = true;
      event.dataTransfer?.clearData(); // Clean up
    }
  }

  function handleFilesUploaded(uploadedFiles: File[]) {
    // This function would be called if DocumentUploader emits an event after processing.
    // For now, DocumentUploader might directly interact with chat context or emit messages.
    console.log('Files uploaded via DocumentUploader:', uploadedFiles);
    showDocumentUploader.value = false; // Close uploader after "upload"
    // Potentially send a message to the chat indicating files were processed/attached.
    // sendMessage(asUnredacted(`Attached ${uploadedFiles.length} file(s).`));
  }

  // Handler for prompt selection from PromptLibrary
  function handlePromptSelected(promptText: string) {
    sendMessage(asUnredacted(promptText)); // Send the selected prompt text as a message
    setViewMode('chat'); // Switch back to chat view
  }

  // Handler for template selection from TemplateLibrary
  function handleTemplateSelected(templateContent: string) {
    sendMessage(asUnredacted(templateContent)); // Send the selected template content
    setViewMode('chat'); // Switch back to chat view
  }

  function handleQuickPromptSelectedFromHeader(promptText: string) {
    const chatStore = useChatStore();
    chatStore.setSelectedTextForInput(promptText);
    // Optionally, ensure chat view is active if a quick prompt is selected
    // setViewMode('chat');
  }

  function handleQuickTemplateSelectedFromHeader(templateContent: string) {
    const chatStore = useChatStore();
    chatStore.setSelectedTextForInput(templateContent); // Use the same store action to populate input
    // setViewMode('chat'); // Ensure chat view is active
  }

  function handleEnhancePrompt(promptToEnhance: string) {
    const suggestions: string[] = [];
    const originalPrompt = promptToEnhance.trim();

    // 1. Role Clarity
    if (!/^(Act as a|You are)/i.test(originalPrompt)) {
      suggestions.push("- Define a role for the AI (e.g., 'Act as a [ROLE]...').");
    }

    // 2. Task Articulation (Simple check for "and" or multiple sentences suggesting sprawl)
    // This is a basic heuristic and might need refinement.
    // const sentences = originalPrompt.split(/[.!?]+\s/).filter(Boolean);
    // if (originalPrompt.toLowerCase().includes(' and ') && sentences.length > 1) {
    //   suggestions.push("- Ensure the task is a single, explicit objective; avoid 'and also' sprawl.");
    // }
    // For now, a general suggestion might be better:
    if (suggestions.length < 1) { // Add if no other major issues found yet, or always add
       suggestions.push("- Ensure the task is a single, explicit objective.");
    }

    // 3. Context Block
    if (!/\[CONTEXT\]|\{\{DATA\}\}|\[YOUR_TEXT_HERE\]/i.test(originalPrompt)) {
      suggestions.push("- Provide necessary context, data, or examples for the AI (e.g., use placeholders like [CONTEXT]).");
    }

    // 4. Constraints
    if (!/\b(word count|words|tone|style|format|structure)\b/i.test(originalPrompt)) {
      suggestions.push("- Specify constraints like word count, tone, or output structure.");
    }

    // 5. Output Format
    if (!/\b(table|JSON|bullet points|list)\b/i.test(originalPrompt)) {
      suggestions.push("- Clearly state the desired output format (e.g., table, JSON, bullet points).");
    }

    // 6. Success Criteria
    if (!/\b(success is|it's good when|evaluate based on|success criteria)\b/i.test(originalPrompt)) {
      suggestions.push("- Include success criteria or a mini-rubric (e.g., 'It's good when...').");
    }

    // 7. Iteration Hook
    if (!/\b(ask for clarification|if unsure|clarify)\b/i.test(originalPrompt)) {
      suggestions.push("- Add an iteration hook (e.g., 'Ask for clarification if anything is unclear.').");
    }

    if (suggestions.length > 0) {
      const suggestionText = "\n\n--- Prompt Enhancement Suggestions ---\n" + suggestions.join("\n");
      const chatStore = useChatStore();
      chatStore.setSelectedTextForInput(originalPrompt + suggestionText);
    } else {
      // If no specific suggestions, maybe provide a generic positive feedback or do nothing
      // For now, if no suggestions, we don't modify the prompt.
      // Alternatively, inform the user the prompt looks good according to basic checks.
      const chatStore = useChatStore();
      chatStore.setSelectedTextForInput(originalPrompt + "\n\n(Basic prompt structure looks reasonable.)");
    }
  }

  // Removed local messages ref and handleSend/handleFiles/handleMic logic
  // as they are now managed by useChatLogic composable.
  </script>

  <style scoped>
    /* Add any panel-specific styles if needed */
  </style>