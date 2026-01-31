import { createApp } from 'vue';
import { createPinia } from 'pinia'; // Import createPinia
import './style.css'; // We'll need to create/configure this for Tailwind
import App from './App.vue'; // We need to create a root App.vue component
import router from './router'; // Import the router

const pinia = createPinia(); // Create the Pinia instance

createApp(App)
  .use(router) // Tell the app to use the router
  .use(pinia) // Tell the app to use Pinia
  .mount('#app');
