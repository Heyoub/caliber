<script setup lang="ts">
import { ref, markRaw } from 'vue'; // Import markRaw
import {
  Sun as SunIcon, // Alias imports
  Clock as ClockIcon,
  Sparkles as SparklesIcon,
  Map as MapIcon,
  Calendar as CalendarIcon,
  Star as StarIcon,
  ArrowDown as ArrowDownIcon,
  MessageSquare as MessageSquareIcon,
  Notebook as NotebookIcon,
  Brain as BrainIcon,
  Bolt as BoltIcon,
  ArrowRight as ArrowRightIcon,
  Split as SplitIcon,
  KeySquare as KeySquareIcon,
  CalendarDays as CalendarDaysIcon, // Alias this one too
  MapPin as MapPinIcon
} from 'lucide-vue-next';
import GlassMorphicCard from '../ui/GlassMorphicCard.vue';
// Removed duplicate/incorrect icon import lines that were here
import GlowPressButton from '../ui/GlowPressButton.vue';
import FeaturesCloud from './FeaturesCloud.vue';
import FeaturesPrem from './FeaturesPrem.vue';
import HowItWorks from './HowItWorks.vue';
import Badge from '@/components/ui/Badge.vue';

// Mark icons as raw
const Sun = markRaw(SunIcon);
const Clock = markRaw(ClockIcon);
const Sparkles = markRaw(SparklesIcon);
const Map = markRaw(MapIcon);
const Calendar = markRaw(CalendarIcon);
const Star = markRaw(StarIcon);
const ArrowDown = markRaw(ArrowDownIcon);
const MessageSquare = markRaw(MessageSquareIcon);
const Notebook = markRaw(NotebookIcon);
const Brain = markRaw(BrainIcon);
const Bolt = markRaw(BoltIcon);
const ArrowRight = markRaw(ArrowRightIcon);
const Split = markRaw(SplitIcon);
const KeySquare = markRaw(KeySquareIcon);
const CalendarDays = markRaw(CalendarDaysIcon);
const MapPin = markRaw(MapPinIcon);

const selectedPath = ref<string | null>(null);
const selectedFeatures = ref<string | null>(null);

function selectPath(path: string) {
  selectedPath.value = path;
  if (path === 'focus') {
    selectedFeatures.value = 'cloud';
  } else if (path === 'ownership') {
    selectedFeatures.value = 'prem';
  }
  // TODO: Implement actual navigation or state update based on path
}

interface TimelineItem {
  time: string;
  title: string;
  description: string;
  icon: any;
  iconColorClass: string;
  timeColorClass: string; // Added for time text color
  bgColor: string;
  borderColor: string;
  glowColor: string;
  status: string;
  accentColorVar: string; // e.g., '--coral-500' for dynamic HSL
}


const timelineItems: TimelineItem[] = [
  {
    time: "9:00 AM",
    title: "Morning Focus",
    description: "The Crown bar captures your \"draft an email to Sarah about project timeline\" command. AI drafts it while you review today's priorities in your Focus quadrant.",
    icon: Sun,
    iconColorClass: "text-[hsl(var(--coral-400))]",
    timeColorClass: "text-[hsl(var(--coral-300))]",
    bgColor: "bg-slate-600/70",
    borderColor: "border-[hsla(var(--coral-500),0.6)]", // Slightly stronger border
    glowColor: "bg-[hsla(var(--coral-500),0.4)]",    // Slightly stronger glow
    status: "Complete",
    accentColorVar: "--coral-500"
  },
  {
    time: "1:00 PM",
    title: "Client Meeting Notes",
    description: "Your meeting notes are automatically linked to the client contact. One click transforms discussion points into actionable tasks and project milestones.",
    icon: Notebook,
    iconColorClass: "text-[hsl(var(--teal-400))]",
    timeColorClass: "text-[hsl(var(--teal-300))]",
    bgColor: "bg-slate-600/70",
    borderColor: "border-[hsla(var(--teal-500),0.6)]",
    glowColor: "bg-[hsla(var(--teal-500),0.4)]",
    status: "In Progress",
    accentColorVar: "--teal-500"
  },
  {
    time: "4:00 PM",
    title: "Deep Work Session",
    description: "While working on a complex proposal, the Dock keeps essential context visible—the client's requirements, similar past projects, and resource availability.",
    icon: Sparkles,
    iconColorClass: "text-[hsl(var(--mint-400))]",
    timeColorClass: "text-[hsl(var(--mint-300))]",
    bgColor: "bg-slate-600/70",
    borderColor: "border-[hsla(var(--mint-500),0.6)]",
    glowColor: "bg-[hsla(var(--mint-500),0.4)]",
    status: "Upcoming",
    accentColorVar: "--mint-500"
  },
  {
    time: "6:00 PM",
    title: "Day Wrap-Up",
    description: "AI summarizes the day's progress and proposes tomorrow's focus areas. Your completed project work automatically generates invoices for approval.",
    icon: Map,
    iconColorClass: "text-[hsl(var(--lavender-400))]",
    timeColorClass: "text-[hsl(var(--lavender-300))]",
    bgColor: "bg-slate-600/70",
    borderColor: "border-[hsla(var(--lavender-500),0.6)]",
    glowColor: "bg-[hsla(var(--lavender-500),0.4)]",
    status: "Upcoming",
    accentColorVar: "--lavender-500"
  }
];

// Refs for potential future animation/intersection observer logic
const titleRef = ref<HTMLDivElement | null>(null);
const timelineRef = ref<HTMLDivElement | null>(null);

</script>

<template>
  <section id="journey" class="relative py-24 bg-transparent text-white">
     <!-- Animated path connector (visual only) -->
    <div class="absolute left-1/2 top-0 bottom-0 w-1.5 transform -translate-x-1/2 z-0 opacity-60">
      <div
        class="w-full h-full bg-gradient-to-b from-[hsl(var(--teal-500))] via-[hsl(var(--lavender-500))] to-[hsl(var(--coral-500))]"
      ></div>
    </div>

    <div class="container relative z-10 mx-auto px-4">
      <div
        ref="titleRef"
        class="max-w-3xl mx-auto mb-20 text-center"
      >
        <!-- "A Day in the Life" Badge -->
        <Badge
          appearance="frosted-glass"
          colorScheme="neutral-slate"
          :icon="CalendarDays"
          iconSize="sm"
          text="A Day in the Life"
          textSize="sm"
          class="mb-6"
        />
        
        <h2 class="text-4xl md:text-5xl font-bold mb-8 text-slate-50">
          Your Day with <span class="text-gradient bg-clip-text text-transparent bg-gradient-to-r from-[hsl(var(--coral-400))] to-[hsl(var(--coral-500))]">ForgeStack</span>
        </h2>
        
        <p class="text-xl text-slate-200 max-w-2xl mx-auto"> <!-- Slightly lighter text for paragraph -->
          Experience how a cognitive-first platform transforms your daily workflow by adapting to your natural thought process.
        </p>
      </div>
      
      <!-- Timeline Container -->
      <div class="max-w-4xl mx-auto" ref="timelineRef">
        <div class="relative">
          <!-- Timeline line -->
          <div class="absolute left-5 md:left-8 top-0 bottom-0 w-1 bg-gradient-to-b from-[hsla(var(--coral-600),0.5)] via-[hsla(var(--teal-600),0.5)] to-[hsla(var(--lavender-600),0.5)] rounded-full"></div>
          
          <div class="space-y-16">
            <!-- Timeline Item Loop -->
            <div
              v-for="(item, index) in timelineItems"
              :key="item.title"
              class="flex flex-col md:flex-row items-start group"
            >
              <!-- Icon and Time Column -->
              <div class="md:w-20 flex-shrink-0 mb-4 md:mb-0 flex md:flex-col items-center relative z-10">
                <div :class="`w-12 h-12 rounded-full ${item.bgColor} flex items-center justify-center border-2 border-solid ${item.borderColor} group-hover:scale-110 transition-transform duration-300 relative shadow-lg`">
                  <div :class="`absolute -inset-1.5 ${item.glowColor} rounded-full opacity-0 group-hover:opacity-50 blur-xl transition-opacity duration-500`"></div> <!-- Enhanced glow -->
                  <component :is="item.icon" class="h-5 w-5 relative z-10" :class="item.iconColorClass" :size="20" />
                </div>
                <div class="ml-3 md:ml-0 md:mt-3 flex flex-col items-center">
                  <span class="text-sm font-semibold" :class="item.timeColorClass">{{ item.time }}</span> <!-- Colored time -->
                  <!-- Status Badge -->
                  <span
                    :class="`mt-1.5 px-2.5 py-1 rounded-full text-xs font-semibold shadow-md border-solid ${
                      item.status === 'Complete'
                        ? 'bg-[hsla(var(--teal-500),0.25)] text-[hsl(var(--teal-300))] border border-[hsla(var(--teal-500),0.4)]'
                        : item.status === 'In Progress'
                        ? 'bg-[hsla(var(--coral-500),0.25)] text-[hsl(var(--coral-300))] border border-[hsla(var(--coral-500),0.4)]'
                        : 'bg-[hsla(var(--slate-600),0.25)] text-[hsl(var(--slate-300))] border border-[hsla(var(--slate-500),0.4)]'
                    }`"
                  >
                    {{ item.status }}
                  </span>
                </div>
              </div>
              
              <!-- Timeline Item Card -->
              <div class="md:ml-6 w-full bg-slate-600/85 backdrop-blur-lg hover:bg-slate-600/95 shadow-xl hover:shadow-2xl transition-all duration-300 border border-solid border-slate-700 hover:border-[hsla(var(item.accentColorVar),0.5)] rounded-xl overflow-hidden">
                <!-- Card Top Accent (using item's accent color) -->
                <div class="h-1.5 w-full opacity-80" :style="{ backgroundColor: `hsl(var(${item.accentColorVar}))` }"></div>
                <div class="p-6">
                  <h3 class="text-xl font-semibold mb-3 text-slate-100">{{ item.title }}</h3>
                  <p class="text-slate-300 mb-5 text-base">{{ item.description }}</p>
                  
                  <!-- Inner Mockup Content -->
                  <div class="bg-slate-850/80 rounded-lg p-4 border border-solid border-slate-700/80 shadow-inner"> 
                    <!-- Mockup for "Morning Focus" -->
                    <div v-if="index === 0" class="flex flex-col space-y-3">
                      <div class="h-14 bg-[hsla(var(--coral-700),0.3)] rounded-md w-full relative overflow-hidden border border-solid border-[hsla(var(--coral-500),0.5)] flex items-center px-4 shadow-md"> 
                        <MessageSquare class="h-5 w-5 text-[hsl(var(--coral-300))] mr-3 flex-shrink-0" :size="20" />
                        <span class="font-medium text-sm text-[hsl(var(--coral-200))]">/email Sarah about timeline</span>
                      </div>
                      <div class="flex flex-col md:flex-row space-y-3 md:space-y-0 md:space-x-3 mt-2">
                        <div class="w-full md:w-2/3 h-44 md:h-48 bg-slate-750/80 border border-solid border-slate-600/80 shadow-sm p-4 relative rounded-md"> 
                          <div class="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r from-[hsl(var(--coral-600))] to-[hsl(var(--coral-500))]"></div>
                          <div class="w-full h-3.5 bg-slate-600/80 rounded mb-2.5"></div>
                          <div class="w-3/4 h-3.5 bg-slate-600/80 rounded mb-2.5"></div>
                          <div class="w-1/2 h-3.5 bg-slate-600/80 rounded mb-5"></div>
                          <div class="w-full h-3.5 bg-slate-600/80 rounded mb-2.5"></div>
                          <div class="w-5/6 h-3.5 bg-slate-600/80 rounded"></div>
                        </div>
                        <div class="w-full md:w-1/3 h-44 md:h-48 bg-slate-750/80 border border-solid border-slate-600/80 shadow-sm rounded-md flex flex-col items-center justify-center p-4"> 
                          <div class="text-center">
                            <Calendar class="h-8 w-8 text-[hsl(var(--coral-400))] mx-auto mb-2.5" :size="32" />
                            <h4 class="text-sm font-medium text-slate-100">Today's Focus</h4>
                            <div class="mt-3 space-y-2">
                              <div class="h-3.5 w-28 bg-slate-600/80 rounded-full mx-auto"></div>
                              <div class="h-3.5 w-20 bg-slate-600/80 rounded-full mx-auto"></div>
                              <div class="h-3.5 w-24 bg-slate-600/80 rounded-full mx-auto"></div>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                    <!-- Mockup for "Client Meeting Notes" -->
                    <div v-else-if="index === 1" class="flex flex-col space-y-3">
                      <div class="flex items-center mb-2">
                        <div class="w-10 h-10 rounded-full bg-[hsla(var(--teal-700),0.35)] mr-3 flex items-center justify-center border border-solid border-[hsla(var(--teal-500),0.5)] shadow-md"> 
                          <Notebook class="h-5 w-5 text-[hsl(var(--teal-300))] flex-shrink-0" :size="20" />
                        </div>
                        <div>
                          <div class="text-sm font-medium text-slate-100">Acme Corp Meeting Notes</div>
                          <div class="text-xs text-slate-300">Just now • Auto-linked to Client #342</div>
                        </div>
                      </div>
                      <div class="h-36 bg-slate-750/80 rounded-md border border-solid border-slate-600/80 shadow-sm p-3">
                        <div class="w-full h-3.5 bg-slate-600/80 rounded mb-2.5"></div>
                        <div class="w-3/4 h-3.5 bg-slate-600/80 rounded mb-2.5"></div>
                        <div class="w-1/2 h-3.5 bg-slate-600/80 rounded mb-4"></div>
                        <div class="flex space-x-2">
                          <span class="px-2.5 py-1 bg-[hsla(var(--teal-500),0.3)] rounded text-xs text-[hsl(var(--teal-300))] border border-solid border-[hsla(var(--teal-500),0.4)]">Deadline</span>
                          <span class="px-2.5 py-1 bg-[hsla(var(--mint-500),0.3)] rounded text-xs text-[hsl(var(--mint-300))] border border-solid border-[hsla(var(--mint-500),0.4)]">Budget</span>
                          <span class="px-2.5 py-1 bg-[hsla(var(--coral-500),0.3)] rounded text-xs text-[hsl(var(--coral-300))] border border-solid border-[hsla(var(--coral-500),0.4)]">Scope</span>
                        </div>
                      </div>
                      <div class="flex justify-end space-x-2">
                        <button class="px-3.5 py-1.5 bg-gradient-to-r from-[hsl(var(--teal-500))] to-[hsl(var(--teal-600))] hover:from-[hsl(var(--teal-400))] hover:to-[hsl(var(--teal-500))] text-white text-xs rounded-md shadow-lg hover:shadow-xl transition-all flex items-center">
                          <ArrowDown class="h-3 w-3 mr-1.5" :size="12" />
                          <span>Transform to Tasks</span>
                        </button>
                      </div>
                    </div>
                    <!-- Mockup for "Deep Work Session" -->
                    <div v-else-if="index === 2" class="flex flex-col space-y-3">
                      <div class="h-40 bg-slate-750/80 rounded-md border border-solid border-slate-600/80 shadow-sm p-3 relative">
                        <div class="absolute inset-0 w-2/3 bg-gradient-to-r from-transparent via-[hsla(var(--mint-500),0.1)] to-[hsla(var(--mint-500),0.2)] opacity-60 rounded-md"></div>
                        <div class="w-full h-3.5 bg-slate-600/80 rounded mb-2.5"></div>
                        <div class="w-3/4 h-3.5 bg-slate-600/80 rounded mb-2.5"></div>
                        <div class="w-1/2 h-3.5 bg-slate-600/80 rounded"></div>
                        <div class="absolute bottom-3 right-3 flex space-x-2">
                          <div class="h-8 w-8 rounded-md bg-[hsla(var(--mint-700),0.4)] border border-solid border-[hsla(var(--mint-500),0.5)] flex items-center justify-center shadow-md">
                            <Sparkles class="h-4 w-4 text-[hsl(var(--mint-300))]" :size="16" />
                          </div>
                          <div class="h-8 w-8 rounded-md bg-[hsla(var(--coral-700),0.4)] border border-solid border-[hsla(var(--coral-500),0.5)] flex items-center justify-center shadow-md">
                            <Notebook class="h-4 w-4 text-[hsl(var(--coral-300))]" :size="16" />
                          </div>
                        </div>
                      </div>
                      <div class="h-14 bg-slate-750/80 rounded-md flex items-center justify-around p-2 border border-solid border-slate-600/80 shadow-sm"> 
                        <div class="flex items-center px-3 py-1.5 bg-slate-600/80 rounded-md shadow-inner border border-solid border-slate-700/70">
                          <div class="w-3 h-3 rounded-full bg-[hsla(var(--coral-500),0.35)] border border-solid border-[hsla(var(--coral-500),0.45)]"></div>
                          <span class="ml-2 text-xs text-slate-200">Context</span>
                        </div>
                        <div class="flex items-center px-3 py-1.5 bg-slate-600/80 rounded-md shadow-inner border border-solid border-slate-700/70">
                          <div class="h-4 w-16 rounded-full bg-[hsla(var(--lavender-500),0.3)] border border-solid border-[hsla(var(--lavender-500),0.4)]"></div>
                          <span class="ml-2 text-xs text-slate-200">Reference</span>
                        </div>
                        <div class="flex items-center px-3 py-1.5 bg-slate-600/80 rounded-md shadow-inner border border-solid border-slate-700/70">
                          <div class="w-3 h-3 rounded-full bg-[hsla(var(--mint-500),0.35)] border border-solid border-[hsla(var(--mint-500),0.45)]"></div>
                          <span class="ml-2 text-xs text-slate-200">Tools</span>
                        </div>
                      </div>
                    </div>
                    <!-- Mockup for "Day Wrap-Up" -->
                    <div v-else-if="index === 3" class="flex flex-col space-y-3">
                      <div class="h-20 bg-[hsla(var(--lavender-700),0.35)] rounded-md w-full border border-solid border-[hsla(var(--lavender-500),0.5)] p-3 flex items-center gap-2 relative overflow-hidden shadow-md">
                        <Map class="h-5 w-5 text-[hsl(var(--lavender-300))] flex-shrink-0" :size="20" />
                        <span class="text-sm font-medium text-[hsl(var(--lavender-200))]">Daily Summary & Tomorrow's Plan</span>
                      </div>
                      <div class="flex flex-col md:flex-row space-y-3 md:space-y-0 md:space-x-3">
                        <div class="w-full md:w-2/3 h-36 bg-slate-750/80 rounded-md border border-solid border-slate-600/80 shadow-sm p-3">
                          <div class="text-xs font-medium text-slate-100 mb-2">Today's Accomplishments</div>
                          <div class="w-full h-3 bg-slate-600/80 rounded mb-2"></div>
                          <div class="w-3/4 h-3 bg-slate-600/80 rounded mb-2"></div>
                          <div class="w-1/2 h-3 bg-slate-600/80 rounded mb-2"></div>
                          <div class="w-5/6 h-3 bg-slate-600/80 rounded"></div>
                        </div>
                        <div class="w-full md:w-1/3 h-36 bg-slate-750/80 rounded-md border border-solid border-slate-600/80 shadow-sm flex flex-col items-center justify-center p-4 relative">
                          <div class="absolute top-0 left-0 right-0 h-1 bg-gradient-to-r from-[hsl(var(--lavender-600))] to-[hsl(var(--lavender-500))]"></div>
                          <div class="flex items-center justify-center mb-2.5">
                            <div class="h-8 w-8 rounded-full bg-[hsla(var(--mint-700),0.45)] border border-solid border-[hsla(var(--mint-500),0.55)] flex items-center justify-center shadow-md">
                              <span class="text-sm font-bold text-[hsl(var(--mint-300))]">$</span>
                            </div>
                          </div>
                          <div class="text-lg font-semibold text-center text-slate-100">
                            2 invoices ready
                          </div>
                          <div class="text-xs text-slate-300 text-center mt-1">
                            $3,240 total
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </section>

  <section id="choice-point" class="relative py-20 pb-24 bg-transparent text-white">
    <!-- Background gradient (subtle, dark) -->
    <div class="absolute inset-0 bg-transparent z-[-1]"></div>
    <!-- Animated path connector (visual only) -->
    <div class="absolute left-1/2 top-0 bottom-0 w-1.5 transform -translate-x-1/2 z-0 opacity-50">
      <div class="w-full h-full bg-gradient-to-b from-[hsl(var(--coral-500))] via-[hsl(var(--lavender-500))] to-[hsl(var(--teal-500))]"></div>
    </div>

    <!-- Content -->
    <div class="container relative z-10 mx-auto px-4">
      <div class="max-w-3xl mx-auto mb-16 text-center">
        <!-- "Your Cognitive Journey" Badge -->
        <Badge
          appearance="frosted-glass"
          colorScheme="neutral-slate"
          :icon="MapPin"
          iconSize="sm"
          text="Your Cognitive Journey"
          textSize="sm"
          class="mb-8"
        />
        
        <h2 class="text-4xl md:text-5xl font-bold mb-8 text-white">
          What <span class="text-gradient bg-clip-text text-transparent bg-gradient-to-r from-[hsl(var(--teal-400))] to-[hsl(var(--mint-400))]">Matters Most</span> to You?
        </h2>
        
        <p class="text-xl text-slate-200 max-w-2xl mx-auto">
          Everyone's cognitive journey is different. Choose the path that resonates with your biggest challenge.
        </p>
      </div>
      
      <div class="flex flex-col md:flex-row gap-10 md:gap-12 max-w-6xl mx-auto">
        <!-- Focus Path Card -->
        <div
          class="md:w-1/2 relative group rounded-xl transition-all duration-300"
          :class="selectedPath === 'focus' ? 'ring-4 ring-offset-2 ring-offset-slate-950 ring-[hsl(var(--mint-500))] shadow-2xl' : 'hover:shadow-xl'"
        >
          <!-- GlassMorphicCard should have its own border visible based on its theme="dark" -->
          <GlassMorphicCard 
            glowColor="mint" 
            hoverEffect 
            theme="dark" 
            class="classNameh-full border border-solid border-slate-700/70 hover:border-[hsl(var(--mint-600))] transition-colors"
          >
            <div class="p-8 text-center h-full flex flex-col">
              <div class="relative inline-block mx-auto mb-6">
                <div class="absolute -inset-2 bg-[hsla(var(--mint-400),0.2)] rounded-full opacity-60 animate-pulse group-hover:opacity-80 transition-opacity"></div>
                <div class="relative z-10 flex items-center justify-center w-20 h-20 bg-gradient-to-br from-[hsl(var(--mint-500))] to-[hsl(var(--mint-600))] rounded-full shadow-lg group-hover:scale-110 transition-transform duration-300">
                  <Brain class="h-10 w-10 text-white" :size="40" />
                </div>
              </div>
              <h3 class="text-2xl md:text-3xl font-bold mb-4 text-slate-100">Focus & Flow</h3>
              <p class="text-slate-300 text-lg mb-6 flex-grow">
                I want to stay in my creative zone with AI assistance that enhances my natural workflow, not dictates it.
              </p>
              <div class="mb-8 space-y-2 text-left text-sm text-slate-300">
                <div class="flex items-start space-x-2">
                  <Bolt class="h-5 w-5 text-[hsl(var(--mint-400))] mt-0.5 flex-shrink-0" :size="20" />
                  <span>AI that anticipates needs</span>
                </div>
                <div class="flex items-start space-x-2">
                  <Bolt class="h-5 w-5 text-[hsl(var(--mint-400))] mt-0.5 flex-shrink-0" :size="20" />
                  <span>Seamless context switching</span>
                </div>
              </div>
              <GlowPressButton
                :color="selectedPath === 'focus' ? 'mint' : 'slate'"
                @click.stop="selectPath('focus')"
                size="md"
                class="w-full"
              >
                <span>Explore This Path</span>
                <ArrowRight class="h-4 w-4 ml-2" :size="16" />
              </GlowPressButton>
            </div>
          </GlassMorphicCard>
        </div>
        
        <!-- Ownership Path Card -->
        <div
          class="md:w-1/2 relative group rounded-xl transition-all duration-300"
          :class="selectedPath === 'ownership' ? 'ring-4 ring-offset-2 ring-offset-slate-950 ring-[hsl(var(--coral-500))] shadow-2xl' : 'hover:shadow-xl'"
        >
          <GlassMorphicCard 
            glowColor="coral" 
            hoverEffect 
            theme="dark" 
            class="classNameh-full border border-solid border-slate-700/70 hover:border-[hsl(var(--coral-600))] transition-colors"
          >
            <div class="p-8 text-center h-full flex flex-col">
              <div class="relative inline-block mx-auto mb-6">
                <div class="absolute -inset-2 bg-[hsla(var(--coral-400),0.2)] rounded-full opacity-60 animate-pulse group-hover:opacity-80 transition-opacity"></div>
                <div class="relative z-10 flex items-center justify-center w-20 h-20 bg-gradient-to-br from-[hsl(var(--coral-500))] to-[hsl(var(--coral-600))] rounded-full shadow-lg group-hover:scale-110 transition-transform duration-300">
                  <KeySquare class="h-10 w-10 text-white" :size="40" />
                </div>
              </div>
              <h3 class="text-2xl md:text-3xl font-bold mb-4 text-slate-100">Owning My Tools</h3>
              <p class="text-slate-300 text-lg mb-6 flex-grow">
                I want to break free from endless subscriptions and own my technology infrastructure with more privacy.
              </p>
              <div class="mb-8 space-y-2 text-left text-sm text-slate-300">
                <div class="flex items-start space-x-2">
                  <Bolt class="h-5 w-5 text-[hsl(var(--coral-400))] mt-0.5 flex-shrink-0" :size="20" />
                  <span>Own your data and tools</span>
                </div>
                <div class="flex items-start space-x-2">
                  <Bolt class="h-5 w-5 text-[hsl(var(--coral-400))] mt-0.5 flex-shrink-0" :size="20" />
                  <span>No more SaaS lock-in</span>
                </div>
              </div>
              <GlowPressButton
                :color="selectedPath === 'ownership' ? 'coral' : 'slate'"
                @click.stop="selectPath('ownership')"
                size="md"
                class="w-full"
              >
                <span>Explore This Path</span>
                <ArrowRight class="h-4 w-4 ml-2" :size="16" />
              </GlowPressButton>
            </div>
          </GlassMorphicCard>
        </div>
      </div>
      
      <!-- Floating decoration elements (dark theme adjusted) -->
      <div class="absolute top-1/4 left-10 w-24 h-24 bg-[hsla(var(--mint-500),0.1)] rounded-full opacity-40 blur-3xl -z-10"></div>
      <div class="absolute bottom-1/4 right-10 w-32 h-32 bg-[hsla(var(--coral-500),0.1)] rounded-full opacity-40 blur-3xl -z-10"></div>

      <!-- Materialized Features Card Section -->
      <div class="mt-20 md:mt-24">
        <transition name="fade-slide" mode="out-in">
          <div v-if="selectedFeatures" :key="selectedFeatures + '-card'" class="w-full max-w-5xl mx-auto">
            <GlassMorphicCard
              :glowColor="selectedFeatures === 'cloud' ? 'mint' : 'coral'"
              theme="dark" 
              className="w-full overflow-hidden shadow-2xl border border-solid border-slate-700/70"
              style="min-height: 300px;"
            >
              <FeaturesCloud v-if="selectedFeatures === 'cloud'" />
              <FeaturesPrem v-else-if="selectedFeatures === 'prem'" />
            </GlassMorphicCard>
          </div>
          <div v-else key="placeholder" class="mt-12 w-full text-center text-slate-300 text-lg font-medium opacity-100 py-10"> <!-- Adjusted text color for dark theme -->
            Select a path above to see tailored features.
          </div>
        </transition>
      </div>
    </div>
    <HowItWorks />
    </section>
</template>

<style scoped>
/* General text gradient if not globally defined */
.text-gradient {
  @apply bg-clip-text text-transparent;
}

/* Transition for features card */
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: opacity 0.4s ease, transform 0.4s ease;
}
.fade-slide-enter-from,
.fade-slide-leave-to {
  opacity: 0;
  transform: translateY(20px);
}

:root {
  --coral-200: 10 80% 80%;
  --coral-300: 10 75% 70%;
  --coral-400: 15 75% 60%;
  --coral-500: 15 85% 50%;
  --coral-600: 20 80% 45%;
  --coral-700: 20 70% 35%;

  --teal-300: 170 65% 60%;
  --teal-400: 175 60% 50%;
  --teal-500: 180 70% 40%;
  --teal-600: 185 65% 35%;
  --teal-700: 185 60% 25%;

  --mint-300: 155 65% 65%;
  --mint-400: 160 60% 55%;
  --mint-500: 165 70% 45%;
  --mint-600: 170 65% 40%;
  --mint-700: 170 60% 30%;

  --lavender-200: 250 70% 85%;
  --lavender-300: 255 65% 75%;
  --lavender-400: 260 60% 65%;
  --lavender-500: 265 70% 55%;
  --lavender-600: 270 65% 50%;
  --lavender-700: 270 60% 40%;

  --slate-50: 220 20% 95%;
  --slate-100: 220 20% 90%;
  --slate-200: 215 20% 85%;
  --slate-300: 215 18% 75%;
  --slate-400: 215 16% 65%;
  --slate-500: 215 15% 55%;
  --slate-600: 220 15% 45%;
  --slate-700: 220 15% 35%;
  --slate-750: 220 15% 30%;
  --slate-800: 222 18% 20%;
  --slate-850: 222 18% 15%;
  --slate-900: 225 20% 10%;
  --slate-950: 225 20% 5%;
}
</style>