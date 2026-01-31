<script setup lang="ts">
import { ref } from 'vue';
import {
  ArrowRight,
  Play,
  MessageSquare,
  Sparkles,
  Rocket
} from 'lucide-vue-next';
import GlowBrandButton from '../ui/GlowBrandButton.vue';

// Assuming UI components need replacement or Vue versions
// import Button from "@/components/ui/button"; // Replaced with <button>
// import Input from "@/components/ui/input"; // Replaced with <input>
// import Label from "@/components/ui/label"; // Replaced with <label>
// import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"; // Replaced with <select>/<option>
// import { Card, CardContent } from "@/components/ui/card"; // Replaced with <div>

const email = ref("");
const companySize = ref("");

// Refs for potential future animation/intersection observer logic
const sectionRef = ref<HTMLElement | null>(null);
const titleRef = ref<HTMLDivElement | null>(null);
const formRef = ref<HTMLDivElement | null>(null);
const linkRef = ref<HTMLDivElement | null>(null);

const handleSubmit = () => {
  // In a real implementation, this would connect to the backend API
  console.log("Form submitted:", { email: email.value, companySize: companySize.value });
  // Reset form
  email.value = "";
  companySize.value = "";
};
</script>

<template>
  <section id="get-started" ref="sectionRef" class="relative mb-0 py-12">
    <!-- Background elements -->
    <div class="absolute inset-0 bg-gradient-to-br from-[#0f172a] via-[#0f172a]/0 to-[#0f172a]/20 opacity-0 z-0"></div>
    
    <!-- Decorative circles -->
    <div class="absolute top-20 right-[10%] w-64 h-64 rounded-full bg-[hsl(var(--coral-500))]/10 opacity-20 blur-3xl z-0"></div> <!-- Changed bg-coral-100 -->
    <div class="absolute bottom-20 left-[10%] w-80 h-80 rounded-full bg-[hsl(var(--teal-500))]/10 opacity-20 blur-3xl z-0"></div> <!-- Changed bg-teal-100 -->
    
    <!-- Main Content -->
    <div class="container relative z-10 mx-0 px-0">
      <div class="max-w-5xl mx-0">
        <div
          ref="titleRef"
          class="text-center m-0"
        >          
          <h2 class="text-4xl md:text-5xl lg:text-6xl font-bold mb- text-white leading-tight">

            <span class="block mb-3">Your mind deserves <span class="text-[hsl(var(--coral-500))] font-medium">better.</span></span>
            <span class="block mb-3">Your business deserves <span class="text-[hsl(var(--blue-500))] font-medium">better.</span></span>
            <span class="inline-block relative">
              <span class="text-[hsl(var(--mint-500))]">Ready to</span>
              <span class="text-[hsl(var(--lavender-500))]"> reclaim it?</span>
              <svg
  width="100%"
  height="12"
  viewBox="0 0 200 8"
  fill="none"
  xmlns="http://www.w3.org/2000/svg"
  class="absolute -bottom-2.5 left-0 right-0 w-full"
>
  <path
    d="M1 5.5C40 1.5 160 1.5 199 5.5"
    stroke="url(#paint0_linear_invitation)"
    stroke-width="4"
    stroke-linecap="round"
    fill="none"
  />
  <defs>
    <linearGradient id="paint0_linear_invitation" x1="1" y1="3" x2="199" y2="3" gradientUnits="userSpaceOnUse">
      <stop stop-color="hsl(var(--coral-500))"/>
      <stop offset="0.33" stop-color="hsl(var(--teal-500))"/>
      <stop offset="0.66" stop-color="hsl(var(--mint-500))"/>
      <stop offset="1" stop-color="hsl(var(--lavender-500))"/>
    </linearGradient>
  </defs>
</svg>
            </span>
          </h2>
          
          <p class="text-xl text-white/80 max-w-2xl mx-auto mt-6 m-0">
          </p>
        </div>
        
        <div class="grid md:grid-cols-5 gap-8 items-start">
          <!-- Form Card - Replaced Card with div -->
          <div
            ref="formRef"
            class="md:col-span-3"
          >
             <div class="rounded-xl overflow-hidden shadow-lg hover:shadow-xl border border-[hsl(var(--coral-500))]/20 bg-slate-800/70 backdrop-blur-xl transition-shadow duration-300">
               <div class="h-2 bg-gradient-to-r from-coral-500 via-teal-500 to-mint-500"></div>
 
               <!-- Replaced CardContent with div -->
               <div class="p-8">        
                 <form @submit.prevent="handleSubmit" class="space-y-6">
                   <div class="mb-4 relative dropdown-wrapper">
                      <label for="email" class="text-sm font-medium text-white/80 mb-1.5 block">Work Email</label>
                      <input
    type="email"
    id="email"
    v-model="email"
    required
    class="w-full px-4 py-3 border bg-slate-800/30 backdrop-blur-xl text-white rounded-lg shadow-[0_0.25rem_0.75rem_rgba(79,209,197,0.15)] border-white/10 border-solid focus:outline-none focus:ring-2 focus:border-teal-500 focus:ring-teal-500/50 transition-all duration-300"
    placeholder="Your email address"
  />
                   </div>
                   
                   <div>
                     <label for="company-size" class="text-sm font-medium text-white/80 mb-1.5 block">Company Size</label>
                     <div class="dropdown-wrapper mb-4 relative">
  <select
    id="company-size"
    v-model="companySize"
    required
    class="dark-select w-full px-4 py-3 border bg-black/80 text-white rounded-lg shadow-[0_0.25rem_0.75rem_rgba(79,209,197,0.15)] border-teal-900/60 border-solid focus:border-teal-500 focus:ring-2 focus:ring-teal-500/50 focus:outline-none appearance-none pr-10 transition-all duration-300 relative z-10"
    style="background-color:rgba(15,23,42,0.95); color:#fff; appearance:none;"
  >
    <option value="" disabled class="text-white/70">Select team size</option>
    <option value="1-10">1-10 employees</option>
    <option value="11-50">11-50 employees</option>
    <option value="51-200">51-200 employees</option>
    <option value="201+">201+ employees</option>
  </select>
  <span class="custom-chevron pointer-events-none absolute right-4 top-1/2 -translate-y-1/2 flex items-center justify-center z-20">
    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>
  </span>
  <div class="absolute inset-0 laser-glow opacity-0 pointer-events-none transition-opacity duration-300 rounded-lg z-10"></div>
</div>
                   </div>
                   
                  <div class="relative w-full h-[3.5rem] flex items-center justify-center">
                    <GlowBrandButton
                      variant="coral"
                      :fullWidth="true"
                      type="submit"
                    >
                      <span class="flex items-center justify-center">
                        <h3 class="text-2xl font-bold mb-0">Private Beta, Coming July 2025!</h3>
                        <ArrowRight class="ml-2 h-5 w-5" :size="20" /> 
                      </span>
                    </GlowBrandButton>
                  </div>
                 </form>
                 
                 <div class="mt-6 pt-6 border-t border-[hsl(var(--coral-500))]/10">
                   <div class="flex items-center justify-center gap-1 text-white/60 text-sm">
                     <Sparkles class="h-4 w-4" :size="16" />
                     <span>14-day free trial • No credit card required</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
          
          <!-- Testimonial / Features -->
          <div
            class="md:col-span-2"
          >
            <div class="bg-slate-800/70 backdrop-blur-xl rounded-xl border border-[hsl(var(--mint-500))]/20 p-6 shadow-lg mb-6">
              <div class="flex items-start mb-4">
                <div class="rounded-full bg-[hsl(var(--mint-500))]/20 p-3 mr-4 border border-[hsl(var(--mint-500))]/30">
                  <MessageSquare class="h-6 w-6 text-mint-700" :size="24" />
                </div>
                <div>
                  <h4 class="text-lg font-semibold text-white">We Want To Hear From You!</h4>
                  <p class="text-white/70 italic mt-3">
                    "Join the ForgeStack Beta — help us shape smarter, human-first tools for small business. Your feedback builds the future."
                  </p>
                  <div class="mt-3 text-sm text-white/60">
                    — The ForgeStack Team
                  </div>
                </div>
              </div>
            </div>
            
            <div ref="linkRef" class="space-y-4">
              <a href="#demo" class="flex items-center p-4 bg-slate-800/70 backdrop-blur-xl rounded-xl border border-[hsl(var(--lavender-500))]/20 text-white hover:border-[hsl(var(--lavender-500))]/40 hover:shadow-lg transition-all duration-300 group">
                <div class="rounded-full bg-[hsl(var(--lavender-500))]/20 p-2 mr-4 border border-[hsl(var(--lavender-500))]/30">
                  <Play class="h-5 w-5 text-lavender-700" :size="20" />
                </div>
                <div>
                  <div class="font-medium">Watch a demo</div>
                  <div class="text-sm text-white/60">See ForgeStack in action (4 min)</div>
                </div>
                <ArrowRight class="ml-auto h-5 w-5 opacity-0 group-hover:opacity-100 group-hover:translate-x-1 transition-all" :size="20" />
              </a>
              
              <a href="#contact" class="flex items-center p-4 bg-slate-800/70 backdrop-blur-xl rounded-xl border border-[hsl(var(--coral-500))]/20 text-white hover:border-[hsl(var(--coral-500))]/40 hover:shadow-lg transition-all duration-300 group">
                <div class="rounded-full bg-[hsl(var(--coral-500))]/20 p-2 mr-4 border border-[hsl(var(--coral-500))]/30">
                  <MessageSquare class="h-5 w-5 text-coral-700" :size="20" />
                </div>
                <div>
                  <div class="font-medium">Contact our team</div>
                  <div class="text-sm text-white/60">Get personalized guidance</div>
                </div>
                <ArrowRight class="ml-auto h-5 w-5 opacity-0 group-hover:opacity-100 group-hover:translate-x-1 transition-all" :size="20" />
              </a>
            </div>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.dropdown-wrapper {
  z-index: 0;
}
.dark-select {
  background-color: rgba(15,23,42,0.95)!important;
  color: #fff!important;
  appearance: none!important;
  -webkit-appearance: none!important;
  -moz-appearance: none!important;
  border: 1.5px solid #134e4a;
}
.dark-select:focus {
  border: 2px solid #4FD1C5;
  box-shadow: 0 0 0 2px #4fd1c588;
}
.dark-select option {
  background: #0f172a;
  color: #fff;
}

.dropdown-wrapper:hover .laser-glow,
.dropdown-wrapper:focus-within .laser-glow {
  opacity: 0.15;
}

.laser-glow {
  background: radial-gradient(circle at center, #4FD1C5 0%, transparent 70%);
  box-shadow: 0 0 15px 2px rgba(79, 209, 197, 0.5), inset 0 0 8px rgba(79, 209, 197, 0.3);
}

.custom-dropdown-wrapper {
}
/* .custom-dropdown {
  backdrop-filter: blur(12px);
  border: 1px solid rgba(255,255,255,0.10);
  border-radius: 0.75rem;
  color: #fff;
  box-shadow: 0 4px 24px 0 rgba(79,209,197,0.09);
  transition: border 0.3s, box-shadow 0.3s;
} */
/* .custom-dropdown:focus {
  border: 1.5px solid #4FD1C5;
  box-shadow: 0 0 0 2px #4fd1c588;
} */
/* .custom-dropdown option {
  color: #fff;
  border-radius: 0.5rem;
  transition: background 0.2s;
} */
/* .custom-dropdown option:hover, .custom-dropdown option:focus {
  background: rgba(79,209,197,0.09);
  color: #fff;
} */
/* .custom-chevron {
  height: 2rem;
  width: 2rem;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
  z-index: 2;
} */
/* Hide default arrow for most browsers */
/* .custom-dropdown::-ms-expand {
  display: none;
} */
/* .custom-dropdown::-webkit-outer-spin-button,
.custom-dropdown::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
} */
/* .custom-dropdown {
  -moz-appearance: none;
  -webkit-appearance: none;
  appearance: none;
} */
.bg-glassy-filled {
  backdrop-filter: blur(12px);
  box-shadow: 0 4px 24px 0 rgba(79,209,197,0.09);
  transition: background 0.3s, box-shadow 0.3s;
}
.brand-gradient-text {
  background: linear-gradient(90deg,
    hsl(var(--coral-500)),
    hsl(var(--teal-500)) 33%,
    hsl(var(--mint-500)) 66%,
    hsl(var(--lavender-500)));
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
  color: transparent;
}
.text-navy-100 {
  color: hsl(var(--navy-100));
}
.placeholder\:text-navy-200::placeholder {
  color: hsl(var(--navy-200));
  opacity: 1;
}

</style>