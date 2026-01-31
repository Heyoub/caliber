<template>
  <div class="chat-panel">
    <div class="chat-header">
      <h2>Emi Ai</h2>
    </div>
    
    <div class="chat-body">
      <div class="message-container">
        <div class="message system-message">
          Hi, I'm Emi. Welcome! How can I assist you today?
        </div>
        
        <div class="message user-message">
          Can you tell me more about this service?
        </div>
        
        <div class="message system-message">
          Of course! Council Chat is our AI assistant that helps you manage your business workflows.
          <button @click="showPopup" class="info-button">
            Learn More
          </button>
        </div>
      </div>
    </div>
    
    <div class="chat-input">
      <input type="text" placeholder="Type your message..." />
      <button class="send-button">Send</button>
    </div>
    
    <!-- Chat Popup Component -->
    <ChatPopup
      ref="popupRef"
      title="Hi, I'm Emi. Welcome!"
      disclaimer="This AI assistant uses industry best practices to protect your data. All interactions are encrypted and we do not store your conversations without explicit consent."
    >
      <div class="popup-content">
        <img src="/IMG/CouncilChat.png" alt="Council Chat Logo" class="popup-logo" />
        
        <p>
          This is a standalone version of the AI assistant from the ForgeStack CRM, demonstrating its powerful capabilities.
        </p>
        
        <p>
          You're looking at a functional demo. For example, the document uploader and analysis tools you see here utilize the same underlying tool-use architecture and functionalities as in the full CRM. This showcases how the AI can interact with various tools to perform complex tasks.
        </p>
        
        <p>
          Essentially, this is a demo of the demo, highlighting the core AI assistance features. We plan to offer this standalone AI as a paid microservice in the future.
        </p>
        
        <button @click="closePopup" class="action-button">
          Got it!
        </button>
      </div>
    </ChatPopup>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import ChatPopup from './ChatPopup.vue';

const popupRef = ref<InstanceType<typeof ChatPopup> | null>(null);

const showPopup = () => {
  popupRef.value?.showPopup();
};

const closePopup = () => {
  popupRef.value?.closePopup();
};
</script>

<style scoped>
.chat-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: hsl(var(--slate-900));
  border-left: 1px solid hsla(var(--slate-700), 0.5);
  position: relative;
}

.chat-header {
  padding: 16px;
  border-bottom: 1px solid hsla(var(--slate-700), 0.5);
  background-color: hsla(var(--slate-850), 0.9);
}

.chat-header h2 {
  margin: 0;
  color: hsl(var(--slate-100));
  font-size: 1.2rem;
  font-weight: 600;
}

.chat-body {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

.message-container {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.message {
  padding: 12px 16px;
  border-radius: 12px;
  max-width: 85%;
  line-height: 1.5;
}

.system-message {
  align-self: flex-start;
  background-color: hsla(var(--slate-800), 0.8);
  color: hsl(var(--slate-200));
  border-bottom-left-radius: 4px;
}

.user-message {
  align-self: flex-end;
  background-color: hsla(var(--teal-600), 0.3);
  color: hsl(var(--slate-100));
  border-bottom-right-radius: 4px;
}

.chat-input {
  padding: 16px;
  display: flex;
  gap: 8px;
  border-top: 1px solid hsla(var(--slate-700), 0.5);
  background-color: hsla(var(--slate-850), 0.9);
}

.chat-input input {
  flex: 1;
  padding: 12px 16px;
  border-radius: 8px;
  border: 1px solid hsla(var(--slate-600), 0.5);
  background-color: hsla(var(--slate-800), 0.8);
  color: hsl(var(--slate-200));
}

.chat-input input:focus {
  outline: none;
  border-color: hsla(var(--teal-500), 0.5);
}

.send-button {
  padding: 0 16px;
  border-radius: 8px;
  border: none;
  background-color: hsl(var(--teal-500));
  color: white;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.2s;
}

.send-button:hover {
  background-color: hsl(var(--teal-600));
}

.info-button {
  display: inline-block;
  margin-top: 8px;
  padding: 4px 12px;
  border-radius: 4px;
  border: 1px solid hsla(var(--teal-500), 0.5);
  background-color: transparent;
  color: hsl(var(--teal-400));
  font-size: 0.85rem;
  cursor: pointer;
  transition: all 0.2s;
}

.info-button:hover {
  background-color: hsla(var(--teal-500), 0.1);
}

.popup-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
}

.popup-logo {
  width: 80px;
  height: 80px;
  margin-bottom: 16px;
}

.popup-content p {
  margin-bottom: 12px;
  font-size: 0.95rem;
  line-height: 1.5;
}

.action-button {
  margin-top: 16px;
  padding: 8px 24px;
  border-radius: 8px;
  border: none;
  background-color: hsl(var(--teal-500));
  color: white;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.2s;
}

.action-button:hover {
  background-color: hsl(var(--teal-600));
}

:root {
  --slate-100: 220 20% 90%;
  --slate-200: 215 20% 85%;
  --slate-600: 215 14% 45%;
  --slate-700: 220 15% 35%;
  --slate-800: 222 18% 20%;
  --slate-850: 222 18% 15%;
  --slate-900: 225 20% 10%;
  --teal-400: 175 60% 50%;
  --teal-500: 180 70% 40%;
  --teal-600: 180 65% 35%;
}
</style>
