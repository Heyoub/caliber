<template>
  <div>
    <ChatPanel theme="emi" />
  </div>
</template>

<script setup lang="ts">
import { ref, type Ref } from 'vue';
import ChatPanel from '@/ai/components/common/ChatPanel.vue';
import type { Message } from '@/ai/types/chat.types'; 

const isSending: Ref<boolean> = ref(false);
const currentView: Ref<string> = ref('EmiAppChat'); 
const messages: Ref<Message[]> = ref([]); 

const handleSendMessage = async (messageContent: string) => {
  if (!messageContent.trim()) return;

  const userMessage: Message = {
    id: Date.now().toString(), 
    role: 'user',
    content: messageContent,
    timestamp: new Date().toISOString(),
    type: 'text'
  };
  messages.value.push(userMessage);

  isSending.value = true;
  try {
    setTimeout(() => {
      const assistantMockMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: 'assistant',
        content: `Emi received: "${messageContent}". (Mock response)`,
        timestamp: new Date().toISOString(),
        type: 'text'
      };
      messages.value.push(assistantMockMessage);
      isSending.value = false;
    }, 1000);

  } catch (error) {
    console.error("Error sending message:", error);
    const errorMessage: Message = {
      id: (Date.now() + 1).toString(),
      role: 'assistant',
      content: 'Sorry, Emi encountered an error.',
      timestamp: new Date().toISOString(),
      type: 'text',
      isError: true 
    };
    messages.value.push(errorMessage);
    isSending.value = false;
  }
};
</script>

<style scoped>

</style>