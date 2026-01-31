<template>
  <!-- Abstract Dashboard UI Illustration -->
  <div class="relative py-16">
    <div class="relative h-96 md:h-[33rem] mb-4"> <!-- Increased height slightly -->
      <div class="absolute top-0 left-1/2 transform -translate-x-1/2 w-full max-w-5xl perspective-1000">
        <!-- Stylized glow behind the UI -->
        <div class="absolute -inset-6 bg-gradient-to-br from-[hsla(var(--teal-500),0.2)] via-[hsla(var(--mint-500),0.2)] to-[hsla(var(--lavender-500),0.2)] blur-3xl opacity-75 rounded-3xl animate-pulse-slow"></div>

        <!-- The UI mockup -->
        <div class="relative w-full h-96 md:h-[33rem] bg-gradient-to-br from-slate-700/80 to-slate-900/90 rounded-2xl shadow-2xl overflow-hidden animate-float border border-slate-700 border-solid backdrop-blur-lg">
          <!-- Top bar with app controls -->
          <div class="absolute top-0 left-0 w-full h-12 bg-slate-600/70 flex items-center px-4 border-b border-slate-700 border-solid backdrop-blur-md">
            <div class="flex items-center space-x-2">
              <div class="w-3 h-3 rounded-full bg-[hsl(var(--coral-500))] shadow-[0_0_0.5rem_hsla(var(--coral-500),0.7)] animate-pulse-slow"></div>
              <div class="w-3 h-3 rounded-full bg-[hsl(var(--mint-500))] shadow-[0_0_0.5rem_hsla(var(--mint-500),0.7)] animate-pulse-slow animation-delay-100"></div>
              <div class="w-3 h-3 rounded-full bg-[hsl(var(--teal-500))] shadow-[0_0_0.5rem_hsla(var(--teal-500),0.7)] animate-pulse-slow animation-delay-200"></div>
            </div>
            <div class="ml-6 flex-1 flex justify-center">
              <!-- Mock URL Bar -->
              <div class="w-2/3 h-7 bg-slate-600/60 rounded-md flex items-center justify-center px-2 border border-slate-600/70 border-solid">
                <div class="w-full h-5 bg-slate-600/70 rounded-sm"></div>
              </div>
            </div>
            <div class="flex space-x-2">
              <!-- Mock Action Icon -->
              <div class="w-8 h-8 rounded-full flex items-center justify-center text-[hsl(var(--mint-400))] hover:bg-slate-600/60 transition-colors cursor-pointer">
                <svg class="h-4 w-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon>
                </svg>
              </div>
            </div>
          </div>

          <!-- Main layout: Sidebar + Content Area -->
          <div class="absolute top-12 left-0 w-full h-[calc(100%-6rem)] flex"> <!-- Adjusted height for top and bottom bar -->
            <!-- Left sidebar - sleeker design -->
            <div class="w-1/5 bg-slate-600/50 p-4 border-r border-slate-700 border-solid backdrop-blur-sm">
              <!-- Mock Logo/User Area -->
              <div class="flex items-center gap-3 mb-8">
                <div class="w-10 h-10 rounded-lg bg-gradient-to-br from-[hsl(var(--coral-500))] to-[hsl(var(--coral-600))] shadow-[0_0_0.9375rem_hsla(var(--coral-500),0.5)] border border-[hsla(var(--coral-400),0.5)] flex items-center justify-center">
                  <!-- Placeholder Icon -->
                  <svg class="w-5 h-5 text-white/80" fill="currentColor" viewBox="0 0 20 20"><path d="M10 9a3 3 0 100-6 3 3 0 000 6zm-7 9a7 7 0 1114 0H3z"></path></svg>
                </div>
                <div class="w-3/5 h-5 bg-slate-600/80 rounded-md"></div>
              </div>
              <!-- Mock Navigation Links -->
              <div class="space-y-3.5">
                <div class="w-full h-9 rounded-md bg-slate-600/80 hover:bg-slate-600/80 transition-colors duration-200 px-3 flex items-center border border-transparent hover:border-slate-500/50">
                  <div class="w-full h-3.5 bg-slate-500/60 rounded-sm"></div>
                </div>
                <!-- Active Nav Item -->
                <div class="w-full h-9 rounded-md bg-gradient-to-r from-[hsla(var(--coral-600),0.6)] to-[hsla(var(--coral-500),0.5)] flex items-center px-3 shadow-[0_0_1.125rem_hsla(var(--coral-500),0.4)] border border-[hsla(var(--coral-400),0.6)]">
                  <div class="w-2 h-2 rounded-full bg-white/70 mr-2.5"></div>
                  <div class="w-2/3 h-3.5 bg-[hsla(var(--coral-300),0.9)] rounded-sm"></div>
                </div>
                <div class="w-full h-9 rounded-md bg-slate-600/80 hover:bg-slate-600/80 transition-colors duration-200 px-3 flex items-center border border-transparent hover:border-slate-500/50">
                   <div class="w-4/5 h-3.5 bg-slate-500/60 rounded-sm"></div>
                </div>
                <div class="w-full h-9 rounded-md bg-slate-600/80 hover:bg-slate-600/80 transition-colors duration-200 px-3 flex items-center border border-transparent hover:border-slate-500/50">
                   <div class="w-3/5 h-3.5 bg-slate-500/60 rounded-sm"></div>
                </div>
                <div class="w-full h-9 rounded-md bg-slate-600/60 hover:bg-slate-600/70 transition-colors duration-200 px-3 flex items-center opacity-70 border border-transparent hover:border-slate-500/50">
                   <div class="w-full h-3.5 bg-slate-500/50 rounded-sm"></div>
                </div>
                 <div class="w-full h-9 rounded-md bg-slate-600/60 hover:bg-slate-600/70 transition-colors duration-200 px-3 flex items-center opacity-70 border border-transparent hover:border-slate-500/50">
                   <div class="w-4/5 h-3.5 bg-slate-500/50 rounded-sm"></div>
                </div>
              </div>
            </div>

            <!-- Main content - more engaging visualization -->
            <div class="w-4/5 bg-slate-800/60 p-5 backdrop-blur-sm overflow-y-auto"> <!-- Added p-5 for a bit more padding -->
              <div class="grid grid-cols-2 gap-5"> <!-- Increased gap slightly -->
                <!-- Left Column Cards -->
                <div class="space-y-5">
                  <!-- Card Header Mock -->
                  <div class="h-11 bg-gradient-to-r from-[hsla(var(--teal-600),0.5)] to-[hsla(var(--teal-700),0.4)] rounded-lg flex items-center px-4 border border-[hsla(var(--teal-500),0.3)] shadow-lg">
                    <div class="w-1/2 h-4 bg-slate-200/20 rounded-md"></div>
                  </div>
                  <!-- Card 1 -->
                  <div class="h-44 bg-slate-600/70 rounded-lg shadow-xl p-4 border border-slate-700 border-solid hover:border-slate-600/80 transition-all duration-300">
                    <div class="h-5 w-3/5 bg-slate-600/70 rounded-md mb-3.5"></div>
                    <div class="flex space-x-2.5 mb-3.5">
                      <div class="h-3.5 w-1/4 bg-[hsla(var(--teal-500),0.4)] rounded-sm"></div>
                      <div class="h-3.5 w-1/5 bg-[hsla(var(--coral-500),0.4)] rounded-sm"></div>
                    </div>
                    <div class="space-y-3 p-2.5 bg-slate-600/50 rounded-md border border-slate-600/60 border-solid">
                      <div class="h-2.5 w-full bg-slate-500/50 rounded-full"></div>
                      <div class="h-2.5 w-2/3 bg-gradient-to-r from-[hsla(var(--teal-500),0.6)] to-[hsla(var(--teal-500),0.3)] rounded-full"></div>
                      <div class="h-2.5 w-4/5 bg-slate-500/50 rounded-full"></div>
                      <div class="h-2.5 w-1/2 bg-gradient-to-r from-[hsla(var(--coral-500),0.6)] to-[hsla(var(--coral-500),0.3)] rounded-full"></div>
                    </div>
                  </div>
                  <!-- Card 2 -->
                  <div class="h-44 bg-slate-600/70 rounded-lg shadow-xl p-4 border border-slate-700 border-solid hover:border-slate-600/80 transition-all duration-300">
                    <div class="h-5 w-3/5 bg-slate-600/70 rounded-md mb-3.5"></div>
                    <div class="flex space-x-2.5 mb-3.5">
                      <div class="h-3.5 w-1/4 bg-[hsla(var(--mint-500),0.4)] rounded-sm"></div>
                      <div class="h-3.5 w-1/5 bg-[hsla(var(--lavender-500),0.4)] rounded-sm"></div>
                    </div>
                     <div class="space-y-3 p-2.5 bg-slate-600/50 rounded-md border border-slate-600/60 border-solid">
                        <div class="h-2.5 w-full bg-slate-500/50 rounded-full"></div>
                        <div class="h-2.5 w-1/3 bg-gradient-to-r from-[hsla(var(--mint-500),0.6)] to-[hsla(var(--mint-500),0.3)] rounded-full"></div>
                        <div class="h-2.5 w-4/5 bg-slate-500/50 rounded-full"></div>
                        <div class="h-2.5 w-2/5 bg-gradient-to-r from-[hsla(var(--lavender-500),0.6)] to-[hsla(var(--lavender-500),0.3)] rounded-full"></div>
                      </div>
                  </div>
                </div>
                <!-- Right Column Card (Chart) -->
                <div class="space-y-5">
                  <!-- Card Header Mock -->
                  <div class="h-11 bg-gradient-to-r from-[hsla(var(--mint-600),0.5)] to-[hsla(var(--mint-700),0.4)] rounded-lg flex items-center px-4 border border-[hsla(var(--mint-500),0.3)] shadow-lg">
                    <div class="w-1/2 h-4 bg-slate-200/20 rounded-md"></div>
                  </div>
                  <!-- Chart Card -->
                  <div class="h-[calc(100%-4.25rem)] bg-slate-600/70 rounded-lg shadow-xl p-4 border border-slate-700 border-solid hover:border-slate-600/80 transition-all duration-300">
                    <div class="h-5 w-3/5 bg-slate-600/70 rounded-md mb-5"></div> <!-- Increased margin bottom -->
                    <div class="flex h-[calc(100%-5rem)] relative"> <!-- Adjusted height for title and axis -->
                      <!-- Chart Bars -->
                      <div class="absolute bottom-0 left-[2%] w-[22%] h-1/4 bg-gradient-to-t from-[hsl(var(--teal-500))] to-[hsl(var(--teal-400))] rounded-t-md animate-chart-bar border-t-2 border-[hsl(var(--teal-300))] shadow-[0_-0.3125rem_0.9375rem_hsla(var(--teal-500),0.3)]"></div>
                      <div class="absolute bottom-0 left-[26%] w-[22%] h-2/4 bg-gradient-to-t from-[hsl(var(--coral-500))] to-[hsl(var(--coral-400))] rounded-t-md animate-chart-bar animation-delay-100 border-t-2 border-[hsl(var(--coral-300))] shadow-[0_-0.3125rem_0.9375rem_hsla(var(--coral-500),0.3)]"></div>
                      <div class="absolute bottom-0 left-[50%] w-[22%] h-3/4 bg-gradient-to-t from-[hsl(var(--mint-500))] to-[hsl(var(--mint-400))] rounded-t-md animate-chart-bar animation-delay-200 border-t-2 border-[hsl(var(--mint-300))] shadow-[0_-0.3125rem_0.9375rem_hsla(var(--mint-500),0.3)]"></div>
                      <div class="absolute bottom-0 left-[74%] w-[22%] h-[95%] bg-gradient-to-t from-[hsl(var(--lavender-500))] to-[hsl(var(--lavender-400))] rounded-t-md animate-chart-bar animation-delay-300 border-t-2 border-[hsl(var(--lavender-300))] shadow-[0_-0.3125rem_0.9375rem_hsla(var(--lavender-500),0.3)]"></div>

                      <!-- Interactive Data Point on Tallest Bar -->
                      <div class="absolute bottom-[calc(95%-0.75rem)] left-[calc(74%+11%-0.75rem)]">
                        <div class="w-6 h-6 bg-[hsla(var(--lavender-300),0.4)] rounded-full shadow-xl border-2 border-[hsl(var(--lavender-300))] flex items-center justify-center animate-pulse backdrop-blur-sm">
                          <div class="w-2 h-2 bg-[hsl(var(--lavender-100))] rounded-full"></div>
                        </div>
                      </div>

                      <!-- Axis Labels -->
                      <div class="absolute -bottom-7 left-0 w-full flex justify-around text-[0.65rem] text-slate-300 font-medium">
                        <div>Q1</div>
                        <div>Q2</div>
                        <div>Q3</div>
                        <div>Q4</div>
                      </div>

                      <!-- Data Label -->
                      <div class="absolute top-[calc(5%-0.25rem)] left-[calc(74%+11%-1.75rem)] text-sm font-semibold text-white bg-[hsla(var(--lavender-500),0.7)] px-2.5 py-1 rounded-md shadow-lg border border-[hsla(var(--lavender-300),0.5)] backdrop-blur-sm">
                        84%
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
          <!-- Bottom dock -->
          <div class="absolute bottom-0 left-0 w-full h-12 bg-slate-600/70 border-t border-slate-700 border-solid flex items-center justify-center backdrop-blur-md z-10">
            <div class="flex space-x-6"> <!-- Increased spacing -->
              <!-- Mock Dock Icons -->
              <div class="w-10 h-7 rounded-md bg-gradient-to-br from-[hsla(var(--coral-600),0.7)] to-[hsla(var(--coral-500),0.6)] hover:from-[hsla(var(--coral-500),0.8)] hover:to-[hsla(var(--coral-400),0.7)] transition-all duration-200 shadow-lg hover:shadow-[0_0_1.25rem_hsla(var(--coral-500),0.5)] border border-[hsla(var(--coral-400),0.4)] transform hover:scale-110 cursor-pointer"></div>
              <div class="w-10 h-7 rounded-md bg-gradient-to-br from-[hsla(var(--teal-600),0.7)] to-[hsla(var(--teal-500),0.6)] hover:from-[hsla(var(--teal-500),0.8)] hover:to-[hsla(var(--teal-400),0.7)] transition-all duration-200 shadow-lg hover:shadow-[0_0_1.25rem_hsla(var(--teal-500),0.5)] border border-[hsla(var(--teal-400),0.4)] transform hover:scale-110 cursor-pointer"></div>
              <div class="w-10 h-7 rounded-md bg-gradient-to-br from-[hsla(var(--mint-600),0.7)] to-[hsla(var(--mint-500),0.6)] hover:from-[hsla(var(--mint-500),0.8)] hover:to-[hsla(var(--mint-400),0.7)] transition-all duration-200 shadow-lg hover:shadow-[0_0_1.25rem_hsla(var(--mint-500),0.5)] border border-[hsla(var(--mint-400),0.4)] transform hover:scale-110 cursor-pointer"></div>
              <div class="w-10 h-7 rounded-md bg-gradient-to-br from-[hsla(var(--lavender-600),0.7)] to-[hsla(var(--lavender-500),0.6)] hover:from-[hsla(var(--lavender-500),0.8)] hover:to-[hsla(var(--lavender-400),0.7)] transition-all duration-200 shadow-lg hover:shadow-[0_0_1.25rem_hsla(var(--lavender-500),0.5)] border border-[hsla(var(--lavender-400),0.4)] transform hover:scale-110 cursor-pointer"></div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
export default {
  name: 'AbstractDashboardV2' // Renamed to reflect iteration
}
</script>

<style scoped>
:root {
  --teal-100: 175 70% 80%;
  --teal-300: 170 65% 60%;
  --teal-400: 175 60% 50%;
  --teal-500: 180 70% 40%;
  --teal-600: 185 65% 35%;
  --teal-700: 190 60% 30%;

  --mint-100: 150 70% 85%;
  --mint-300: 155 65% 65%;
  --mint-400: 160 60% 55%;
  --mint-500: 165 70% 45%;
  --mint-600: 170 65% 40%;
  --mint-700: 175 60% 35%;

  --lavender-100: 250 70% 90%;
  --lavender-300: 255 65% 75%;
  --lavender-400: 260 60% 65%;
  --lavender-500: 265 70% 55%;
  --lavender-600: 270 65% 50%;
  --lavender-700: 275 60% 45%;

  --coral-100: 0 80% 80%;
  --coral-300: 5 80% 70%;
  --coral-400: 10 75% 60%;
  --coral-500: 15 85% 50%;
  --coral-600: 20 80% 45%;
  --coral-700: 25 75% 40%;

  --slate-100: 215 25% 95%;
  --slate-200: 215 20% 85%;
  --slate-300: 215 20% 75%;
  --slate-400: 215 18% 65%;
  --slate-500: 215 16% 55%;
  --slate-600: 220 15% 45%;
  --slate-700: 220 15% 30%;
  --slate-700: 222 18% 20%;
  --slate-900: 225 20% 12%;
}

.perspective-1000 {
  perspective: 1000px;
}

/* Float animation with subtle rotate for dashboard */
@keyframes floatCard {
  0% {
    transform: translateY(0px) rotateX(2.5deg) rotateY(-1.5deg) scale(0.97);
  }
  50% {
    transform: translateY(-12px) rotateX(1deg) rotateY(1.5deg) scale(1);
  }
  100% {
    transform: translateY(0px) rotateX(2.5deg) rotateY(-1.5deg) scale(0.97);
  }
}

.animate-float {
  animation: floatCard 12s ease-in-out infinite; /* Slightly longer duration */
  transform-style: preserve-3d;
}

/* Pulse animation for key elements */
@keyframes pulse {
  0%, 100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.65; /* Slightly more noticeable pulse */
    transform: scale(1.1);
  }
}

.animate-pulse {
  animation: pulse 2.2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

.animate-pulse-slow {
  animation: pulse 4.5s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

/* Chart bar animation */
@keyframes chartBar {
  0% {
    opacity: 0;
    transform: scaleY(0);
  }
  100% {
    opacity: 1;
    transform: scaleY(1);
  }
}

.animate-chart-bar {
  animation: chartBar 1.3s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
  transform-origin: bottom;
}

.animation-delay-100 {
  animation-delay: 0.15s; /* Slightly adjusted delays */
}
.animation-delay-200 {
  animation-delay: 0.3s;
}
.animation-delay-300 {
  animation-delay: 0.45s;
}

/* Custom scrollbar for the main content area if needed */
.overflow-y-auto::-webkit-scrollbar {
  width: 8px; /* Slightly wider */
}
.overflow-y-auto::-webkit-scrollbar-track {
  background: hsla(var(--slate-700), 0.7); /* Darker track */
  border-radius: 4px;
}
.overflow-y-auto::-webkit-scrollbar-thumb {
  background: hsla(var(--slate-600), 0.8); /* More visible thumb */
  border-radius: 4px;
  border: 1px solid hsla(var(--slate-700), 0.5); /* Subtle border for thumb */
}
.overflow-y-auto::-webkit-scrollbar-thumb:hover {
  background: hsla(var(--slate-500), 0.9);
}
</style>