<template>
  <div class="w-full min-h-screen bg-gradient-to-br from-lavender-500/20 via-teal-500/20 to-coral-500/40 dark:from-lavender-500/40 dark:via-teal-500/30 dark:to-coral-500/40 text-white overflow-x-hidden">

    <!-- Hero Section: The Small Business Tech Compass -->
    <section class="relative h-[80vh] md:h-[85vh] flex items-center justify-center text-center perspective-1000 overflow-hidden bg-noise">
      <!-- Enhanced Background Glows -->
      <div class="absolute -inset-12 bg-gradient-to-br from-purple-500/20 via-teal-500/[.15] to-coral-500/20 blur-3xl opacity-60 rounded-[4rem] animate-pulse-slow-deep"></div>
      <div class="absolute top-1/4 left-1/4 w-72 h-72 bg-teal-400/10 rounded-full blur-2xl animate-float-subtle opacity-50"></div>
      <div class="absolute bottom-1/4 right-1/4 w-64 h-64 bg-coral-400/10 rounded-full blur-2xl animate-float-subtle animation-delay-300 opacity-50"></div>

      <div class="relative z-10 p-4">
        <!-- Central Animated Orb & Title -->
        <div class="relative flex flex-col items-center justify-center mb-8 animate-float-hero">
          <div class="w-40 h-40 md:w-52 md:h-52 rounded-full bg-gradient-to-tr from-purple-600/80 via-teal-500/70 to-coral-600/80 shadow-2xl flex items-center justify-center relative overflow-hidden ring-2 ring-white/10">
            <div class="absolute inset-0 flex items-center justify-center">
              <div class="w-20 h-20 md:w-28 md:h-28 rounded-full bg-white/10 backdrop-blur-md animate-pulse-orb"></div>
            </div>
            <!-- Guiding Star/Network SVG -->
            <svg class="absolute inset-0 w-full h-full animate-spin-very-slow opacity-50 scale-100 text-white/70" viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
              <defs>
                <clipPath id="heroSvgLaserClip">
                  <rect width="100%" height="100%" rx="24" ry="24"/> {/* 1.5rem assuming 16px base */}
                </clipPath>
                <filter id="ctoHeroGlow" x="-50%" y="-50%" width="200%" height="200%">
                  <feGaussianBlur stdDeviation="1" result="coloredBlur"/>
                  <feMerge><feMergeNode in="coloredBlur"/><feMergeNode in="SourceGraphic"/></feMerge></filter>
              </defs>
              <g filter="url(#ctoHeroGlow)" clip-path="url(#heroSvgLaserClip)">
                <path d="M32 4 L32 60" stroke="currentColor" stroke-width="1" stroke-linecap="round" opacity="0.7"/>
                <path d="M4 32 L60 32" stroke="currentColor" stroke-width="1" stroke-linecap="round" opacity="0.7"/>
                <path d="M12 12 L52 52" stroke="currentColor" stroke-width="0.75" stroke-linecap="round" opacity="0.5"/>
                <path d="M12 52 L52 12" stroke="currentColor" stroke-width="0.75" stroke-linecap="round" opacity="0.5"/>
                <circle cx="32" cy="32" r="2.5" fill="currentColor" opacity="0.9"/>
                <circle cx="32" cy="32" r="5" stroke="currentColor" stroke-width="0.5" opacity="0.6"/>
              </g>
            </svg>
          </div>
        </div>
        
        <h1 class="text-4xl sm:text-5xl md:text-6xl font-bold mb-4 text-slate-50 drop-shadow-lg">
          Stop Fighting Your Tech. <br class="hidden sm:block" /> <span class="text-gradient-animated">Make It Your Co-Pilot.</span>
        </h1>
        <p class="text-lg md:text-xl text-slate-300 max-w-2xl mx-auto mb-10 opacity-90">
          Fractional CTO guidance designed for the <span class="font-semibold text-teal-300">realities of small business</span>—not enterprise fantasies.
        </p>
        <GlowPressButton href="#intro" size="lg" color="primary" class="px-8 py-3.5">
          Discover How
          <ArrowDown class="ml-2 h-5 w-5" />
        </GlowPressButton>
      </div>
    </section>

    <!-- Section 1: Breaking The Entrepreneurial Myth (Your Ebook Intro) -->
    <section id="intro" class="py-20 md:py-28 bg-slate-900/30 backdrop-blur-sm border-y border-solid border-white/5 border-[0.0625rem]">
      <div class="container mx-auto px-6 md:px-4">
        <div class="max-w-3xl mx-auto text-center">
          <div class="inline-flex items-center gap-2 px-4 py-1.5 bg-slate-800/50 rounded-full mb-6 border border-solid border-slate-700">
            <Zap class="h-4 w-4 text-coral-400" />
            <p class="text-sm font-medium text-slate-300">Let's Get Real About Growth</p>
          </div>
          <h2 class="text-3xl md:text-4xl font-bold text-slate-100 mb-6">
            Your Business Isn't a Machine. It's an <span class="text-teal-300">Ecosystem.</span>
          </h2>
          <p class="text-lg text-slate-300 mb-6 leading-relaxed">
            Most business advice treats your company like a predictable machine: pull levers, follow checklists, profit. Sound familiar? That's because it's the standard playbook. <strong class="text-slate-100">And for small businesses, it's often wrong.</strong>
          </p>
          <p class="text-lg text-slate-300 mb-8 leading-relaxed">
            You're not running a giant corporation with siloed departments. Your business is a living, breathing ecosystem—messy, interconnected, and constantly evolving. Trying to manage this complexity with rigid, mechanical solutions is like <strong class="text-coral-300">trying to grow a garden with a toaster manual.</strong>
          </p>
          <p class="text-xl font-semibold text-slate-100">
            It's time for an approach that understands <span class="underline decoration-purple-400 decoration-2 underline-offset-4">your unique reality.</span>
          </p>
        </div>
      </div>
    </section>

    <!-- Section 2: The Real Problem (Mental Models & Feedback Loops) -->
    <section id="real-problem" class="py-20 md:py-28 bg-slate-950/50 bg-noise relative overflow-hidden">
      <div class="absolute inset-0 opacity-5 pointer-events-none">
        <img src="/IMG/axisra-tech-animation.svg" alt="Tech Background" class="w-full h-full object-cover opacity-10 mix-blend-overlay"/>
      </div>
      <div class="container mx-auto px-6 md:px-4 relative z-10">
        <div class="max-w-3xl mx-auto text-center mb-16">
          <div class="inline-flex items-center gap-2 px-4 py-1.5 bg-slate-800/50 rounded-full mb-6 border border-solid border-slate-700">
            <SearchCode class="h-4 w-4 text-mint-400" />
            <p class="text-sm font-medium text-slate-300">Diagnosing the Disconnect</p>
          </div>
          <h2 class="text-3xl md:text-4xl font-bold text-slate-100 mb-6">
            The <span class="text-mint-300">Hidden Operating System</span> That's Holding You Back
          </h2>
          <p class="text-lg text-slate-300 leading-relaxed">
            Every decision you make is shaped by your mental models of how business *should* work. But what if those models are outdated or misaligned with your small business reality? You end up fighting universal patterns instead of leveraging them.
          </p>
        </div>

        <div class="grid md:grid-cols-2 gap-8 lg:gap-12 items-start">
          <!-- Card 1: The Rearview Mirror Problem -->
          <GlassMorphicCard theme="dark" glowColor="coral" class="p-6 md:p-8 rounded-xl border-solid border-coral-500/30 hover:border-coral-500/60 transition-colors duration-300">
            <div class="flex items-center mb-4">
              <div class="p-3 rounded-lg bg-gradient-to-br from-coral-500/20 to-coral-600/30 mr-4 shadow-lg">
                <Radar class="h-6 w-6 text-coral-300" />
              </div>
              <h3 class="text-xl font-semibold text-slate-100">Driving Blind: The Dashboard Delusion</h3>
            </div>
            <p class="text-slate-300 text-sm mb-4 leading-relaxed">
              Dashboards show what <strong class="text-coral-300">already happened</strong>. That's like driving by only looking in the rearview mirror. You need a nervous system that senses patterns *before* they hit your bottom line.
            </p>
            <ul class="space-y-2 text-sm">
              <li class="flex items-start"><CheckCircle class="h-4 w-4 text-coral-400 mr-2 mt-0.5 shrink-0"/>Lagging indicators hide emerging threats.</li>
              <li class="flex items-start"><CheckCircle class="h-4 w-4 text-coral-400 mr-2 mt-0.5 shrink-0"/>Missed opportunities to adapt proactively.</li>
              <li class="flex items-start"><CheckCircle class="h-4 w-4 text-coral-400 mr-2 mt-0.5 shrink-0"/>Decisions based on stale, incomplete data.</li>
            </ul>
          </GlassMorphicCard>

          <!-- Card 2: The Noise vs. Signal Problem -->
          <GlassMorphicCard theme="dark" glowColor="teal" class="p-6 md:p-8 rounded-xl border-solid border-teal-500/30 hover:border-teal-500/60 transition-colors duration-300">
            <div class="flex items-center mb-4">
              <div class="p-3 rounded-lg bg-gradient-to-br from-teal-500/20 to-teal-600/30 mr-4 shadow-lg">
                <Filter class="h-6 w-6 text-teal-300" />
              </div>
              <h3 class="text-xl font-semibold text-slate-100">Signal Through the Noise</h3>
            </div>
            <p class="text-slate-300 text-sm mb-4 leading-relaxed">
              Small businesses are drowning in data but starving for <strong class="text-teal-300">meaning</strong>. The key is to track leading indicators and create fast learning cycles.
            </p>
            <ul class="space-y-2 text-sm">
              <li class="flex items-start"><CheckCircle class="h-4 w-4 text-teal-400 mr-2 mt-0.5 shrink-0"/>Customer sentiment shifts before sales drop.</li>
              <li class="flex items-start"><CheckCircle class="h-4 w-4 text-teal-400 mr-2 mt-0.5 shrink-0"/>Weekly experiments beat quarterly plans.</li>
              <li class="flex items-start"><CheckCircle class="h-4 w-4 text-teal-400 mr-2 mt-0.5 shrink-0"/>Contradictions in data often hide goldmines.</li>
            </ul>
          </GlassMorphicCard>
        </div>
      </div>
    </section>

    <!-- Section 3: Our Approach: Your Business's True Operating System -->
    <section id="our-approach" class="py-20 md:py-28 bg-slate-900/70 backdrop-blur-md border-y border-solid border-white/5 border-[0.0625rem]">
      <div class="container mx-auto px-6 md:px-4">
        <div class="max-w-3xl mx-auto text-center mb-16">
           <div class="inline-flex items-center gap-2 px-4 py-1.5 bg-slate-800/50 rounded-full mb-6 border border-solid border-slate-700">
            <Settings2 class="h-4 w-4 text-purple-400" />
            <p class="text-sm font-medium text-slate-300">The ForgeStack Blueprint</p>
          </div>
          <h2 class="text-3xl md:text-4xl font-bold text-slate-100 mb-6">
            Building Your Business's <span class="text-purple-300">Adaptive Operating System</span>
          </h2>
          <p class="text-lg text-slate-300 leading-relaxed">
            Forget rigid frameworks. We help you install a dynamic OS for your small business—one that fosters clarity, connection, and intelligent action. This isn't about adding more tools; it's about making your existing and new technology work *for* you, cohesively.
          </p>
        </div>

        <div class="grid lg:grid-cols-3 gap-8">
          <!-- Pillar 1: Strategic Clarity -->
          <div class="p-6 bg-slate-850/60 rounded-xl border border-solid border-slate-700/70 shadow-xl hover-lift">
            <div class="flex items-center mb-4">
              <div class="p-3.5 rounded-lg bg-gradient-to-br from-teal-500/25 to-teal-600/35 mr-4 shadow-lg">
                <Compass class="h-7 w-7 text-teal-300" />
              </div>
              <div>
                <h3 class="text-xl font-semibold text-slate-100">Strategic Clarity</h3>
                <p class="text-xs text-teal-400">Goals That Actually Guide</p>
              </div>
            </div>
            <p class="text-slate-300 text-sm mb-4 leading-relaxed">
              No more dusty mission statements. We help you define <strong class="text-slate-100">recursive goals</strong> that adapt to your changing landscape, ensuring your team has constrained autonomy to make smart decisions.
            </p>
            <ul class="space-y-1.5 text-sm text-slate-300">
              <li class="flex items-center"><Dot class="text-teal-400 mr-1 shrink-0"/>SMART goals for foundational focus.</li>
              <li class="flex items-center"><Dot class="text-teal-400 mr-1 shrink-0"/>Agile OKRs for ambitious stretches.</li>
              <li class="flex items-center"><Dot class="text-teal-400 mr-1 shrink-0"/>KPIs that act as your business's real-time dashboard.</li>
            </ul>
          </div>

          <!-- Pillar 2: Customer Connection -->
          <div class="p-6 bg-slate-850/60 rounded-xl border border-solid border-slate-700/70 shadow-xl hover-lift">
            <div class="flex items-center mb-4">
              <div class="p-3.5 rounded-lg bg-gradient-to-br from-coral-500/25 to-coral-600/35 mr-4 shadow-lg">
                <Users class="h-7 w-7 text-coral-300" />
              </div>
              <div>
                <h3 class="text-xl font-semibold text-slate-100">Customer Connection</h3>
                <p class="text-xs text-coral-400">Marketing That Resonates</p>
              </div>
            </div>
            <p class="text-slate-300 text-sm mb-4 leading-relaxed">
              Understand your market and competition without a six-figure budget. We focus on <strong class="text-slate-100">message-market fit</strong> and cost-effective marketing systems that build lasting value.
            </p>
            <ul class="space-y-1.5 text-sm text-slate-300">
              <li class="flex items-center"><Dot class="text-coral-400 mr-1 shrink-0"/>Practical market research techniques.</li>
              <li class="flex items-center"><Dot class="text-coral-400 mr-1 shrink-0"/>Content mapping to the customer journey.</li>
              <li class="flex items-center"><Dot class="text-coral-400 mr-1 shrink-0"/>Building a minimum viable marketing system.</li>
            </ul>
          </div>

          <!-- Pillar 3: Cognitive AI & Automation -->
          <div class="p-6 bg-slate-850/60 rounded-xl border border-solid border-slate-700/70 shadow-xl hover-lift">
            <div class="flex items-center mb-4">
              <div class="p-3.5 rounded-lg bg-gradient-to-br from-mint-500/25 to-mint-600/35 mr-4 shadow-lg">
                <BrainCog class="h-7 w-7 text-mint-300" />
              </div>
              <div>
                <h3 class="text-xl font-semibold text-slate-100">Cognitive AI & Automation</h3>
                <p class="text-xs text-mint-400">Tech That Thinks With You</p>
              </div>
            </div>
            <p class="text-slate-300 text-sm mb-4 leading-relaxed">
              Implement AI that's a true co-pilot, not another complex tool. We focus on AI that understands <strong class="text-slate-100">your specific business context</strong>, automates tedious tasks, and provides actionable insights.
            </p>
            <ul class="space-y-1.5 text-sm text-slate-300">
              <li class="flex items-center"><Dot class="text-mint-400 mr-1 shrink-0"/>AI for process automation & efficiency.</li>
              <li class="flex items-center"><Dot class="text-mint-400 mr-1 shrink-0"/>Custom RAG pipelines for business intelligence.</li>
              <li class="flex items-center"><Dot class="text-mint-400 mr-1 shrink-0"/>Building a "memory" for your business AI.</li>
            </ul>
          </div>
        </div>
      </div>
    </section>
    
    <!-- What We'll Do Together Section (CTOServicesTabs) -->
    <section class="relative overflow-hidden bg-slate-950/70 bg-noise">
      <div class="absolute inset-0 opacity-100 pointer-events-none">
        <img src="/IMG/axisra-tech-animation copy.svg" alt="Tech Background" class="w-full h-full object-cover opacity-90 mix-blend-screen"/>
      </div>
      <div class="relative z-10 w-full backdrop-blur-sm border-t border-b border-solid border-white/5 py-20 md:py-28">
        <div class="container mx-auto px-6 md:px-4 relative z-20">
          <div class="max-w-5xl mx-auto bg-slate-900/50 backdrop-blur-xl border border-solid border-white/10 rounded-2xl shadow-2xl p-6 py-10 md:p-12 relative">
            <div class="max-w-4xl mx-auto text-center mb-12 md:mb-16">
              <div class="inline-flex items-center gap-3 px-4 py-2 bg-slate-800/60 backdrop-blur-md rounded-full mb-6 md:mb-8 border border-solid border-slate-700 shadow-md">
                <Wrench class="text-xl text-teal-400" />
                <p class="text-base font-medium text-slate-200 tracking-wide">Our Fractional CTO Services</p>
              </div>
              <h2 class="text-4xl sm:text-5xl font-bold text-white mb-4 md:mb-6 leading-tight">How We Make It Happen</h2>
              <p class="text-lg md:text-xl text-slate-300 leading-relaxed max-w-3xl mx-auto">
                Strategic technology planning, practical AI implementation, and ongoing guidance—tailored for your small business, without the full-time executive overhead.
              </p>
            </div>
            <CTOServicesTabs class="fade-in-late" />
          </div>
        </div>
      </div>
    </section>

<!-- Consolidated Why Us & What You Get Section -->
    <section class="py-20 md:py-28 bg-gradient-to-b from-slate-900/75 to-transparent backdrop-blur-sm border-y border-transparent border-none border-[0rem]">
      <div class="container mx-auto px-6 md:px-4">
        <div class="max-w-4xl mx-auto text-center mb-16">
          <div class="inline-flex items-center gap-3 px-5 py-2 bg-slate-800/60 backdrop-blur-md rounded-full mb-8 border border-none border-transparent shadow-lg">
            <ShieldCheck class="h-5 w-5 text-mint-400" />
            <p class="text-base font-medium text-slate-200 tracking-wide">Your Partner in Tech Transformation</p>
          </div>
          <h2 class="text-3xl md:text-4xl font-bold text-slate-100 mb-6">
            Unlock Your Business Potential with Strategic Tech
          </h2>
          <p class="text-lg text-slate-300 leading-relaxed max-w-3xl mx-auto">
            We're more than consultants; we're your dedicated tech allies, committed to translating complex technology into tangible business growth and streamlined operations.
          </p>
        </div>

        <div class="grid md:grid-cols-2 gap-8 lg:gap-12 items-stretch">
          <!-- Card 1: Why ForgeStack -->
          <div class="bg-slate-850/70 backdrop-blur-md rounded-xl p-6 md:p-8 border border-solid border-slate-700/80 shadow-xl flex flex-col">
            <div class="flex items-center mb-5">
              <div class="p-3 rounded-lg bg-gradient-to-br from-mint-500/20 to-mint-600/30 mr-4 shadow-lg">
                <Users class="h-7 w-7 text-mint-300" />
              </div>
              <div>
                <h3 class="text-2xl font-semibold text-slate-100">Why ForgeStack?</h3>
                <p class="text-sm text-mint-400">Your Trusted Small Business Tech Partner</p>
              </div>
            </div>
            <p class="text-slate-300 text-sm mb-5 leading-relaxed flex-grow">
              We speak your language, not enterprise jargon. Our strategies are built for the agility and resource realities of small businesses, implementing practical AI and tech that solves real problems.
            </p>
            <div class="space-y-3 text-sm">
              <div v-for="capability in capabilities" :key="capability.title" class="p-3 bg-slate-800/50 rounded-lg border border-slate-700/50 border-solid">
                <div class="flex items-center mb-1.5">
                  <component :is="capability.icon" class="h-5 w-5 mr-2.5 shrink-0" :class="capability.iconTextColor" />
                  <h4 class="font-semibold text-slate-100">{{ capability.title }}</h4>
                </div>
                <p class="text-xs text-slate-300 pl-[2.125rem]">{{ capability.description }}</p>
              </div>
            </div>
          </div>

          <!-- Card 2: The Tangible Benefits -->
          <div class="bg-slate-850/70 backdrop-blur-md rounded-xl p-6 md:p-8 border border-solid border-slate-700/80 shadow-xl flex flex-col">
            <div class="flex items-center mb-5">
              <div class="p-3 rounded-lg bg-gradient-to-br from-purple-500/20 to-purple-600/30 mr-4 shadow-lg">
                <Gift class="h-7 w-7 text-purple-300" />
              </div>
              <div>
                <h3 class="text-2xl font-semibold text-slate-100">What You Get</h3>
                <p class="text-sm text-purple-400">Transforming Tech Burdens into Superpowers</p>
              </div>
            </div>
            <p class="text-slate-300 text-sm mb-5 leading-relaxed flex-grow">
              Experience a clear, actionable tech roadmap, streamlined processes through smart automation, and a scalable technology foundation designed for sustainable growth.
            </p>
            <div class="space-y-3 text-sm">
              <div v-for="benefit in benefits" :key="benefit.title" class="p-3 bg-slate-800/50 rounded-lg border border-slate-700/50 border-solid">
                <div class="flex items-center mb-1.5">
                  <component :is="benefit.icon" class="h-5 w-5 mr-2.5 shrink-0" :class="benefit.iconColorClass" />
                  <h4 class="font-semibold text-slate-100">{{ benefit.title }}</h4>
                </div>
                <p class="text-xs text-slate-300 pl-[2.125rem]">{{ benefit.description }}</p>
              </div>
            </div>
          </div>
        </div>
      </div>
      <div class="pt-16 container mx-auto px-6 md:px-4">
        <div class="max-w-3xl mx-auto text-center mb-16">
          <div class="inline-flex items-center gap-2 px-4 py-1.5 bg-slate-800/50 rounded-full mb-6 border border-solid border-slate-700">
            <Target class="h-4 w-4 text-coral-400" />
            <p class="text-sm font-medium text-slate-300">Are We a Fit?</p>
          </div>
          <h2 class="text-3xl md:text-4xl font-bold text-slate-100 mb-6">
            This Is For Small Business Owners Who...
          </h2>
        </div>
        <div class="max-w-2xl mx-auto bg-slate-850/70 backdrop-blur-md rounded-xl p-8 md:p-10 border border-solid border-slate-700/80 shadow-xl">
          <ul class="space-y-4">
            <li v-for="feature in targetAudienceFeatures" :key="feature" class="flex items-start text-slate-200">
              <Sparkle class="text-teal-400 mr-3 mt-1 h-5 w-5 shrink-0" />
              <span class="flex-1 text-base">{{ feature }}</span>
            </li>
          </ul>
        </div>
      </div>
    </section>

    <!-- Final CTA Section -->
    <section class="w-full bg-slate-700 backdrop-blur-sm border-t border-white/5 border-solid py-12 rounded-t-3xl">
      <div class="container mx-auto max-w-8xl z-20 relative px-0 bg-gradient-to-br from-[#a855f7]/30 via-[#06b6d4]/20 to-[#f59e42]/30 dark:from-[#a855f7]/50 dark:via-[#06b6d4]/30 dark:to-[#f59e42]/50 rounded-3xl">
        <div class="relative flex flex-col md:flex-col items-center justify-between gap-0 bg-transparent backdrop-blur-xl p-0 rounded-[1.5rem] border border-[#34D3EE]/20 shadow-lg fade-in visible overflow-hidden"
          style="box-shadow: 0 0 25px rgba(52, 211, 238, 0.1), 0 0 15px rgba(52, 211, 238, 0.05);">
          <div class="absolute justify-center flex items-center inset-0 opacity-[0.5] z-[-1]">
            <img
              src="/IMG/axisra-tech-animation copy.svg"
              alt="Decorative background animation"
              class="w-full h-full object-cover scale-[1.1] sm:scale-100"
              loading="lazy"
              style="transform-origin: center center;"
            />
          </div>
          <div class="max-w-3xl mx-auto text-center relative z-10 py-4 md:py-8 px-4">
            <h2 class="text-4xl md:text-5xl font-bold text-slate-50 mb-8 leading-tight">
              Ready to Build a <span class="text-gradient-animated">Smarter, Simpler Business?</span>
            </h2>
            <p class="text-lg md:text-xl text-slate-300 mb-10 max-w-2xl mx-auto opacity-90">
              Stop wrestling with tech and start leveraging it. Let's chat about how Fractional CTO leadership and practical AI can transform your small business.
            </p>
            <GlowPressButton href="mailto:contact@forgestack.com" size="lg" color="teal" class="px-10 py-4 text-lg">
              Schedule Your Free Strategy Call
              <ChevronRight class="ml-2 h-5 w-5"/>
            </GlowPressButton>
            <p class="text-sm text-slate-300 mt-6">No obligation, just a conversation about your future.</p>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import GlowPressButton from '../components/core/GlowPressButton.vue'; // Ensure path is correct
import FlipCard from '../components/ui/FlipCard.vue'; // Ensure path is correct
import CTOServicesTabs from '../components/core/CTOServicesTabs.vue'; // Ensure path is correct
import { ArrowDown, Zap, SearchCode, Radar, Filter, Settings2, Compass, Users, BrainCog, Wrench, Target, ShieldCheck, Gift, Sparkle, Dot, CheckCircle, ChevronRight, Package, Scale, Lightbulb } from 'lucide-vue-next'; // Added new icons

onMounted(() => {
  const observerOptions = { root: null, rootMargin: "0px", threshold: 0.1 };
  const observer = new IntersectionObserver((entries) => {
    entries.forEach((entry) => {
      if (entry.isIntersecting) entry.target.classList.add("visible");
    });
  }, observerOptions);
  document.querySelectorAll('.fade-in-late').forEach(el => observer.observe(el));
});

const capabilities = ref([
  {
    title: "Small Business DNA",
    description: "We speak your language, not enterprise jargon. Our strategies are built for the agility and resource realities of small businesses.",
    icon: Package, // Represents tailored solutions
    iconBgColor: "bg-gradient-to-br from-teal-500/20 to-teal-600/30",
    iconTextColor: "text-teal-300",
    features: ["Lean methodologies", "Budget-conscious solutions", "Scalable tech roadmaps", "Clear, actionable advice"]
  },
  {
    title: "Practical AI, Not Hype",
    description: "Cut through the AI noise. We implement AI that solves real small business problems, automates tasks, and provides actual insights.",
    icon: Lightbulb, // Represents ideas/insights from AI
    iconBgColor: "bg-gradient-to-br from-purple-500/20 to-purple-600/30",
    iconTextColor: "text-purple-300",
    features: ["Workflow automation", "Custom AI copilots", "Data-driven decision support", "Demystified AI implementation"]
  },
  {
    title: "Your Strategic Tech Partner",
    description: "More than consultants—we're your dedicated tech co-pilots, invested in your growth and navigating the complexities alongside you.",
    icon: Users, // Represents partnership
    iconBgColor: "bg-gradient-to-br from-coral-500/20 to-coral-600/30",
    iconTextColor: "text-coral-300",
    features: ["Ongoing guidance", "Proactive strategy", "Vendor negotiation support", "Future-proofing your tech"]
  }
]);

const targetAudienceFeatures = ref([
  "Are tired of generic tech advice that doesn't fit their unique business.",
  "Want to leverage AI and modern tech but feel overwhelmed or unsure where to start.",
  "Need strategic tech leadership but can't justify a full-time, six-figure CTO.",
  "Believe their business is an ecosystem, not a machine, and want tech that reflects that.",
  "Are ready to stop fighting their tools and make technology a genuine growth driver."
]);

const benefits = ref([
  {
    title: "Clarity & Direction",
    description: "A clear, actionable tech roadmap that aligns with your specific small business goals and eliminates confusion.",
    icon: Compass,
    iconBgClass: "from-teal-500/20 to-teal-600/30",
    iconColorClass: "text-teal-300"
  },
  {
    title: "Efficiency & Automation",
    description: "Streamlined processes and smart automation, freeing up your time and resources to focus on what you do best.",
    icon: Settings2,
    iconBgClass: "from-mint-500/20 to-mint-600/30",
    iconColorClass: "text-mint-300"
  },
  {
    title: "Sustainable Growth",
    description: "A scalable technology foundation that supports your business's growth today and prepares you for tomorrow's opportunities.",
    icon: Scale,
    iconBgClass: "from-coral-500/20 to-coral-600/30",
    iconColorClass: "text-coral-300"
  }
]);
</script>

<style scoped>
/* Base container and animation styles */
.container { max-width: 1280px; }
.fade-in-late { opacity: 0; transform: translateY(25px); transition: opacity 0.7s ease-out, transform 0.7s ease-out; }
.fade-in-late.visible { opacity: 1; transform: translateY(0); }

/* Hero specific animations */
@keyframes float-hero {
  0%, 100% { transform: translateY(0px); }
  50% { transform: translateY(-15px); }
}
.animate-float-hero { animation: float-hero 7s ease-in-out infinite; }

@keyframes pulse-orb {
  0%, 100% { transform: scale(1); opacity: 0.7; }
  50% { transform: scale(1.08); opacity: 1; }
}
.animate-pulse-orb { animation: pulse-orb 3s ease-in-out infinite; }

@keyframes spin-very-slow { to { transform: rotate(360deg); } }
.animate-spin-very-slow { animation: spin-very-slow 60s linear infinite; }

/* Subtle background element animations */
@keyframes float-subtle {
  0%, 100% { transform: translateY(0px) translateX(0px) rotate(0deg); }
  25% { transform: translateY(-10px) translateX(5px) rotate(15deg); }
  50% { transform: translateY(5px) translateX(-10px) rotate(-10deg); }
  75% { transform: translateY(-5px) translateX(10px) rotate(5deg); }
}
.animate-float-subtle { animation: float-subtle 15s ease-in-out infinite; }

@keyframes pulse-slow-deep {
  0%, 100% { opacity: 0.6; transform: scale(1); }
  50% { opacity: 0.8; transform: scale(1.03); }
}
.animate-pulse-slow-deep { animation: pulse-slow-deep 10s infinite ease-in-out; }

.animation-delay-300 { animation-delay: 0.3s; }
.animation-delay-500 { animation-delay: 0.5s; }

/* Text gradient animation */
.text-gradient-animated {
  background-size: 200% auto;
  background-image: linear-gradient(to right, theme('colors.teal.300') 0%, theme('colors.purple.300') 50%, theme('colors.coral.300') 100%);
  animation: shine 5s linear infinite;
}
@keyframes shine {
  to { background-position: 200% center; }
}

/* Hover lift effect for cards */
.hover-lift {
  transition: transform 0.3s ease-out, box-shadow 0.3s ease-out;
}
.hover-lift:hover {
  transform: translateY(-8px);
  box-shadow: 0 15px 30px hsla(var(--slate-900), 0.2), 0 8px 15px hsla(var(--slate-900),0.15);
}

/* :root color variables removed to use Tailwind theme colors directly from tailwind.config.mjs */
</style>