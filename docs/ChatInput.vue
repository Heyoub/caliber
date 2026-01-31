<template>
    <!-- Apply styling from AIChat.tsx -->
    <div
      :class="[
              'p-4 rounded-t-[1.33rem] border-x-[1.75rem] border-transparent border border-solid backdrop-blur-lg relative', // Changed border-[0.0625rem] to border
              props.theme === 'emi'
                ? 'bg-emi-bg-dark/10 border-emi-primary/30' // Added emi border color
                : 'bg-slate-700/10 border-slate-500/50' // Added default border color
            ]">
      <!-- Top highlight line -->
      <div
        :class="[
          'absolute rounded-t-lg top-0 left-1/4 right-1/4 h-[.5rem]',
          props.theme === 'emi'
            ? 'bg-gradient-to-r from-transparent via-emi-primary/20 to-transparent'
            : 'bg-gradient-to-r from-transparent via-slate-300/20 dark:via-white/10 to-transparent'
        ]"></div>
      <!-- Subtle background gradient -->
      <div
        :class="[
          'absolute inset-0 rounded-t-[1.33rem] pointer-events-none border-emi-primary/30 border-[0.0625rem]', // Added rounded-[1.25rem] here
          props.theme === 'emi' ? 'bg-emi-bg-dark/75' : 'bg-slate-700/75'
        ]"></div>

      <div class="relative z-10 flex flex-col gap-2">
        <!-- Guest Mode Message -->
        <div v-if="!props.isAuthenticated && props.theme === 'default'" class="text-xs text-slate-400 text-center pb-1"> <!-- Only show for default theme if not authenticated -->
          Guest mode. <button @click="handleSignupPrompt"
            :class="[props.theme === 'emi' ? 'text-emi-accent underline hover:text-emi-accent/80' : 'text-teal-400 underline hover:text-teal-300']">Sign up</button> for full features.
        </div>

        <!-- AI Mode Selector -->
        <div class="mode-selector-container flex justify-center items-center space-x-1 p-1 mb-1 rounded-full bg-slate-700/20 dark:bg-slate-800/30 backdrop-blur-md border border-slate-500/30 dark:border-slate-700/50 shadow-md self-center">
          <GlassyButton
            @click="selectedAiMode = 'standard'"
            :active="selectedAiMode === 'standard'"
            class-name="!px-3 !py-1.5 text-xs"
            title="Standard Mode">
            <ChevronDown class="w-4 h-4 mr-1 -ml-1 inline-block" />
            Standard
          </GlassyButton>
          <GlassyButton
            @click="selectedAiMode = 'superThink'"
            :active="selectedAiMode === 'superThink'"
            class-name="!px-3 !py-1.5 text-xs"
            title="SuperThink Mode">
            <Lightbulb class="w-4 h-4 mr-1 -ml-1 inline-block" />
            SuperThink
          </GlassyButton>
          <GlassyButton
            @click="selectedAiMode = 'deepResearch'"
            :active="selectedAiMode === 'deepResearch'"
            class-name="!px-3 !py-1.5 text-xs"
            title="Deep Research Mode">
            <Globe class="w-4 h-4 mr-1 -ml-1 inline-block" />
            DeepResearch
          </GlassyButton>
          <GlassyButton
            @click="selectedAiMode = 'deepWork'"
            :active="selectedAiMode === 'deepWork'"
            class-name="!px-3 !py-1.5 text-xs"
            title="Deep Work Mode">
            <Clock class="w-4 h-4 mr-1 -ml-1 inline-block" />
            DeepWork
          </GlassyButton>
        </div>

        <!-- Heuristic Input Selectors -->
        <div class="heuristic-selectors-container flex flex-wrap justify-center items-center gap-x-4 gap-y-2 p-1 mb-2 text-xs">
          <!-- Intent Selector -->
          <div>
            <label for="intent-selector" class="mr-1 text-slate-300">Intent:</label>
            <select id="intent-selector" v-model="selectedIntent" class="bg-slate-700/50 text-slate-200 border border-slate-600 rounded px-2 py-1 text-xs focus:ring-emi-accent focus:border-emi-accent">
              <option value="GeneralQuery">General Query</option>
              <option value="PlanStrategy">Plan Strategy</option>
              <option value="ExecuteTask">Execute Task</option>
              <option value="Learn">Learn</option>
              <option value="Troubleshoot">Troubleshoot</option>
            </select>
          </div>
          <!-- Scope Selector -->
          <div>
            <label for="scope-selector" class="mr-1 text-slate-300">Scope:</label>
            <select id="scope-selector" v-model="selectedScope" class="bg-slate-700/50 text-slate-200 border border-slate-600 rounded px-2 py-1 text-xs focus:ring-emi-accent focus:border-emi-accent">
              <option value="CurrentMessageOnly">Current Message</option>
              <option value="CurrentConversation">This Conversation</option>
              <option value="CurrentDocument">Active Document</option>
              <option value="EntireProject">Entire Project</option>
              <option value="Global">Global Knowledge</option>
            </select>
          </div>
          <!-- Urgency Selector -->
          <div>
            <label for="urgency-selector" class="mr-1 text-slate-300">Urgency:</label>
            <select id="urgency-selector" v-model="selectedUrgency" class="bg-slate-700/50 text-slate-200 border border-slate-600 rounded px-2 py-1 text-xs focus:ring-emi-accent focus:border-emi-accent">
              <option value="low">Low</option>
              <option value="normal">Normal</option>
              <option value="high">High</option>
            </select>
          </div>
        </div>


        <div class="flex items-center gap-2 w-full">
          <input ref="fileInput" type="file" class="hidden" multiple @change="onFileChange" />
          <button type="button" @click="clearChat" title="Clear Chat"
            :class="['p-2 rounded-[.5rem]', props.theme === 'emi' ? 'hover:bg-emi-accent/10' : 'hover:bg-white/10']">
            <Trash2 :class="['w-5 h-5', props.theme === 'emi' ? 'text-slate-300 hover:text-red-400' : 'text-slate-400 hover:text-red-400']" />
          </button>
          <button type="button" @click="openDocumentUploader" title="Upload Documents"
            :class="['p-2 rounded-[.5rem]', props.theme === 'emi' ? 'hover:bg-emi-accent/10' : 'hover:bg-white/10']">
            <UploadCloud :class="['w-5 h-5', props.theme === 'emi' ? 'text-emi-accent' : 'text-slate-300']" />
          </button>
          <button type="button" @click="$emit('enhance-prompt', messageText)" title="Enhance Prompt"
            :class="['p-2 rounded-[.5rem]', props.theme === 'emi' ? 'hover:bg-emi-accent/10' : 'hover:bg-white/10']">
            <Sparkles :class="['w-5 h-5', props.theme === 'emi' ? 'text-emi-accent' : 'text-purple-400']" />
          </button>
          <input v-model="messageText"
                 v-paste-redact
                 @pii-redacted-paste="handlePastedRedaction"
                 @keydown.enter.prevent="send"
                 type="text"
                 placeholder="Type a message..."
                 :class="[
                    'flex-1 h-[3rem] mt-[.5rem] bg-transparent text-white placeholder-gray-500',
                    props.theme === 'emi' ? 'focus:outline-emi-accent/50 focus:outline-[.25rem]' : 'focus:outline-mint-500/50 focus:outline-[.25rem]'
                 ]" />
          <button
            type="button"
            @click="handleMicClick"
            :class="['p-2 rounded-[.5rem]',
                     props.theme === 'emi' ? 'hover:bg-emi-accent/10' : 'hover:bg-white/10',
                     isRecording ? (props.theme === 'emi' ? 'bg-emi-accent/20' : 'bg-mint-500/20') : '']"
            :aria-pressed="isRecording"
            :title="isRecording ? 'Stop Recording' : 'Start Voice Input'"
          >
            <Mic class="w-5 h-5" :class="isRecording ? (props.theme === 'emi' ? 'text-emi-accent animate-pulse' : 'text-mint-400 animate-pulse') : (props.theme === 'emi' ? 'text-emi-accent' : 'text-slate-300')" />
          </button>
          <RedactionShield class="flex-shrink-0" />
          <button type="button" @click="send" :disabled="isSendDisabled"
            :class="['p-2 rounded-[.5rem]',
                     props.theme === 'emi' ? 'bg-emi-primary/80 hover:bg-emi-primary/60' : 'bg-slate-500/60 hover:bg-slate-500/20',
                     isSendDisabled ? (props.theme === 'emi' ? 'opacity-50 cursor-not-allowed' : 'opacity-50 cursor-not-allowed') : ''
                    ]">
            <Send :class="['w-5 h-5', props.theme === 'emi' ? 'text-white' : 'text-white']" />
          </button>
        </div>
      </div>
    </div>
  </template>
  
  <script setup lang="ts">
  import { ref, watch, computed, type PropType } from 'vue';
  import {
    UploadCloud, Mic, Send, Trash2, Sparkles, XCircle, Loader2,
    Lightbulb, Globe, Clock, ChevronDown // Added Icons for Mode Selector
  } from 'lucide-vue-next';
  import GlassyButton from '@/components/ui/GlassyButton.vue'; // Added GlassyButton import
  import { useVoiceChat, type VoiceChatState } from '@/ai/composables/useVoiceChat';
  import { vPasteRedact, type PasteRedactEventDetail } from '@/ai/directives/vPasteRedact';
  import { redactPII, dedupePlaceholders, type RedactionResult } from '@/ai/lib/redactionUtil';
  import { useRedactionStatus, type RedactionStatusManager } from '@/ai/composables/useRedactionStatus';
  import { useTemporaryRedactionSettings } from '@/ai/composables/useTemporaryRedactionSettings';
  import RedactionShield from './RedactionShield.vue';
  import { type Unredacted, asUnredacted } from '@/ai/types/common.types';
  import { useChatStore } from '@/ai/store/chatStore';
  
  const props = defineProps({
    clearChat: {
      type: Function as PropType<() => void>,
      required: true
    },
    isAuthenticated: {
      type: Boolean,
      required: true
    },
    theme: {
      type: String as PropType<'default' | 'emi'>,
      default: 'default'
    }
  });

  const emit = defineEmits(['send', 'files', 'mic', 'enhance-prompt', 'request-signup', 'open-document-uploader']);
  const messageText = ref('');
  const fileInput = ref<HTMLInputElement | null>(null);
  const statusManager: RedactionStatusManager = useRedactionStatus();
  const { getTemporarilyAllowedTypes } = useTemporaryRedactionSettings();

  // AI Mode Selection
  type AiMode = 'standard' | 'superThink' | 'deepResearch' | 'deepWork';
  const selectedAiMode = ref<AiMode>('standard'); // Default to 'standard'

  // Heuristic Input Selections
  type UserIntent = 'GeneralQuery' | 'PlanStrategy' | 'ExecuteTask' | 'Learn' | 'Troubleshoot';
  type UserScope = 'CurrentMessageOnly' | 'CurrentConversation' | 'CurrentDocument' | 'EntireProject' | 'Global';
  type UserUrgency = 'low' | 'normal' | 'high';

  const selectedIntent = ref<UserIntent>('GeneralQuery');
  const selectedScope = ref<UserScope>('CurrentConversation');
  const selectedUrgency = ref<UserUrgency>('normal');

  const pendingMessage = ref<{ originalText: Unredacted, redactedText: string, redactionResult: RedactionResult } | null>(null);
  
  const {
    isVoiceModeActive,
    voiceState,
    errorMessage: voiceErrorMessage,
    currentTranscript: voiceTranscript,
    toggleVoiceMode,
    stopPlayback,
  } = useVoiceChat();

  function handleMicClick() {
    if (voiceState.value === 'speaking') {
      stopPlayback();
    } else {
      toggleVoiceMode();
    }
  }

  watch(voiceTranscript, (newTranscript) => {
    if (voiceState.value === 'listening' || voiceState.value === 'processing') {
      // messageText.value = newTranscript; 
    }
  });
  
  const isSendDisabled = computed(() => {
    if (isVoiceModeActive.value && voiceState.value !== 'idle' && voiceState.value !== 'error') {
      return true;
    }
    return statusManager.status.value === 'pending-confirmation' || !messageText.value.trim();
  });

  function send() {
    if (statusManager.status.value === 'pending-confirmation') {
      if (pendingMessage.value) {
        statusManager.confirmRedaction();
      }
      return;
    }

    const rawText = messageText.value;
    if (!rawText.trim()) return;

    const temporarilyAllowedTypes = getTemporarilyAllowedTypes();
    const redactionResult: RedactionResult = redactPII(asUnredacted(rawText), { log: true, temporarilyAllowedTypes });

    if (redactionResult.totalRedactions > 0) {
      pendingMessage.value = {
        originalText: asUnredacted(rawText),
        redactedText: dedupePlaceholders(redactionResult.redactedText),
        redactionResult,
      };
      statusManager.setPendingConfirmation();
    } else {
      const finalText = dedupePlaceholders(redactionResult.redactedText);
      emit('send', {
        text: finalText,
        options: {
          aiMode: selectedAiMode.value,
          userStatedIntent: selectedIntent.value,
          userSelectedScope: selectedScope.value,
          userSelectedUrgency: selectedUrgency.value,
        }
      });
      messageText.value = '';
      statusManager.setClean();
      pendingMessage.value = null;
    }
  }

  watch(statusManager.status, (newStatus, oldStatus) => {
    if (oldStatus === 'pending-confirmation' && newStatus === 'redacted' && pendingMessage.value) {
      emit('send', {
        text: pendingMessage.value.redactedText,
        options: {
          aiMode: selectedAiMode.value,
          userStatedIntent: selectedIntent.value,
          userSelectedScope: selectedScope.value,
          userSelectedUrgency: selectedUrgency.value,
        }
      });
      messageText.value = '';
      pendingMessage.value = null;
    } else if (oldStatus === 'pending-confirmation' && newStatus !== 'pending-confirmation' && newStatus !== 'redacted') {
      // Handle cancellation or other status changes if needed
    }
  });

  function handlePastedRedaction(event: Event) {
    const customEvent = event as CustomEvent<PasteRedactEventDetail>;
    if (customEvent.detail) {
      const { fallbackOccurred, totalRedactions } = customEvent.detail.redactionResult;
      if (fallbackOccurred) {
        statusManager.setFallback();
      } else if (totalRedactions > 0) {
        statusManager.setRedacted();
      } else {
        statusManager.setClean();
      }
    }
  }

  function handleSignupPrompt() {
    emit('request-signup');
  }

  function openDocumentUploader() {
    emit('open-document-uploader');
  }

  // Instantiate the chatStore
  const chatStore = useChatStore(); 

  // Watch for selected text from the store to populate the input
  watch(() => chatStore.selectedTextForInput, (newText) => {
    if (newText !== null) {
      messageText.value = newText;
      chatStore.setSelectedTextForInput(null); // Clear after populating
    }
  });

  // Computed property for microphone button icon and state
  const micButtonIcon = computed(() => {
    switch (voiceState.value) {
      case 'listening':
        return Mic;
      case 'processing':
        return Loader2;
      case 'speaking':
        return XCircle;
      case 'error':
        return Mic;
      default: // idle
        return Mic;
    }
  });

  const micButtonTitle = computed(() => {
    switch (voiceState.value) {
      case 'listening':
        return 'Stop Listening';
      case 'processing':
        return 'Processing...';
      case 'speaking':
        return 'Stop AI Speaking';
      case 'error':
        return `Voice Error: ${voiceErrorMessage.value || 'Click to retry'}`;
      default: // idle
        return isVoiceModeActive.value ? 'Stop Voice Mode' : 'Start Voice Input';
    }
  });

  const micButtonClass = computed(() => {
    const baseClasses = ['p-2 rounded-[.5rem]', props.theme === 'emi' ? 'hover:bg-emi-accent/10' : 'hover:bg-white/10'];
    if (voiceState.value === 'listening') {
      baseClasses.push(props.theme === 'emi' ? 'bg-emi-accent/20' : 'bg-mint-500/20');
    } else if (voiceState.value === 'speaking') {
      baseClasses.push(props.theme === 'emi' ? 'bg-red-500/20' : 'bg-red-500/20');
    }
    return baseClasses;
  });

  const micIconClass = computed(() => {
    const base = 'w-5 h-5';
    if (voiceState.value === 'listening' || voiceState.value === 'processing') {
      return [base, props.theme === 'emi' ? 'text-emi-accent animate-pulse' : 'text-mint-400 animate-pulse'];
    }
    if (voiceState.value === 'speaking') {
        return [base, props.theme === 'emi' ? 'text-red-400' : 'text-red-400'];
    }
    return [base, props.theme === 'emi' ? 'text-emi-accent' : 'text-slate-300'];
  });

  // Dummy onFileChange to prevent errors if called, though it's not used with DocumentUploader
  const onFileChange = (event: Event) => {
    console.warn("ChatInput: onFileChange triggered, but file handling should be done by DocumentUploader.", event);
  };
  
  // Computed property for isRecording (based on voiceState)
  const isRecording = computed(() => voiceState.value === 'listening');

  </script>
  
  <style scoped>
  /* (Optional) Additional styling for focus states or responsiveness could be added here */
  </style>