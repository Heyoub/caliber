# Implementation Plan: CALIBER Landing Page

## Overview

Build a single-page marketing site for CALIBER using Astro + Svelte with the SynthBrute aesthetic.

**Status:** ✅ COMPLETE - All tasks finished, site builds successfully and is deployed.

## Tasks

- [x] 1. Project Setup
  - [x] 1.1 Initialize Astro project with Svelte and Tailwind integrations
    - Run `npm create astro@latest caliber-landing`
    - Add `@astrojs/svelte` and `@astrojs/tailwind` integrations
    - Configure `astro.config.mjs`
    - _Requirements: 6.1, 6.2, 6.3_

  - [x] 1.2 Set up design system tokens and global styles
    - Create `src/styles/synthbrute.css` with color palette, typography, glass effects
    - Configure Tailwind with custom colors and utilities
    - Add self-hosted fonts (Space Grotesk, Inter, JetBrains Mono)
    - _Requirements: 5.1, 5.2, 5.4_

  - [x] 1.3 Create base Layout component
    - Create `src/layouts/Layout.astro` with HTML boilerplate
    - Add meta tags, Open Graph, favicon
    - Import global styles
    - _Requirements: 6.1_

- [x] 2. Static Sections (Astro Components)
  - [x] 2.1 Build Navigation component
    - Create `src/components/Nav.astro` with fixed positioning
    - Add glass effect with backdrop blur
    - Include links: Problem, Solution, Architecture, Pricing, GitHub
    - _Requirements: 7.1, 7.2_

  - [x] 2.2 Build Hero section wrapper
    - Create `src/components/Hero.astro`
    - Add headline: "AI agents forget everything. CALIBER fixes that."
    - Add CTA buttons for GitHub and CALIBER Cloud
    - Leave slot for Svelte visualization component
    - _Requirements: 1.3, 1.4_

  - [x] 2.3 Build Problems section
    - Create `src/components/Problems.astro`
    - Add 6 problem cards with brutalist containers
    - Apply neon accent glows and sharp edges
    - _Requirements: 2.1, 2.2, 2.3_

  - [x] 2.4 Build Solutions section
    - Create `src/components/Solutions.astro`
    - Add solution content with code snippets
    - Display key stats (8 crates, 165 tests, etc.)
    - _Requirements: 3.1, 3.3, 3.4_

  - [x] 2.5 Build Architecture section
    - Create `src/components/Architecture.astro`
    - Display ECS architecture diagram
    - Show crate structure visualization
    - _Requirements: 3.3_

  - [x] 2.6 Build Pricing section
    - Create `src/components/Pricing.astro`
    - Create `src/content/pricing.json` as single source of truth
    - Display storage pricing ($1/GB/mo, $10/GB/year)
    - Display hot cache pricing ($0.15/MB/mo)
    - Show unlimited agents, 14-day trial info
    - Add trial CTA and self-host clarification
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6_

  - [x] 2.7 Build Footer component
    - Create `src/components/Footer.astro`
    - Add links to heyoub.dev, GitHub, docs
    - Display AGPL-3.0 license notice
    - _Requirements: 7.3, 7.4_

- [x] 3. Interactive Svelte Islands
  - [x] 3.1 Build Memory Hierarchy Visualization
    - Create `src/components/svelte/MemoryHierarchy.svelte`
    - Implement node structure (Trajectory, Scope, Turn, Artifact, Note)
    - Add sequential fade-in animation on load
    - Add hover states with glass panel descriptions
    - Add connection lines with neon glow pulse
    - _Requirements: 1.1, 1.2_

  - [x] 3.2 Add scroll-based parallax to visualization
    - Implement scroll listener for parallax depth effect
    - Nodes shift at different rates based on depth
    - Use MotionOne for smooth spring animations
    - _Requirements: 1.5, 5.6_

  - [x] 3.3 Build Mobile Navigation
    - Create `src/components/svelte/MobileNav.svelte`
    - Implement hamburger menu toggle
    - Add slide-in menu with glass effect
    - Collapse at 768px breakpoint
    - _Requirements: 8.3_

  - [x] 3.4 Build Code Block component
    - Create `src/components/svelte/CodeBlock.svelte`
    - Add syntax highlighting (Shiki or Prism)
    - Style consistent with SynthBrute aesthetic
    - _Requirements: 3.2_

- [x] 4. Page Assembly and Responsive Design
  - [x] 4.1 Assemble index page
    - Create `src/pages/index.astro`
    - Import and compose all section components
    - Wire up Svelte islands with `client:visible` directive
    - _Requirements: 6.1_

  - [x] 4.2 Implement responsive breakpoints
    - Add mobile styles for all sections
    - Stack pricing cards vertically on mobile
    - Simplify visualization for mobile
    - Test at 320px, 768px, 1024px, 1440px, 2560px
    - _Requirements: 8.1, 8.2, 8.4_

- [x] 5. Deployment and Performance
  - [x] 5.1 Configure Vercel deployment
    - Create `vercel.json` with build settings
    - Set up environment variables if needed
    - Deploy to Vercel
    - _Requirements: 6.5_

  - [x] 5.2 Optimize for Lighthouse 90+
    - Optimize images (WebP/AVIF)
    - Preload critical fonts
    - Minimize JavaScript bundle
    - Run Lighthouse audit and fix issues
    - _Requirements: 6.6_

  - [x] 5.3 Write integration tests

    - Set up Playwright
    - Test page loads, navigation, pricing display
    - Test mobile menu functionality
    - _Requirements: All_

  - [x] 5.4 Write property test for responsive layout

    - Implement Property 1: No horizontal overflow at any viewport width
    - Use fast-check with Playwright
    - **Property 1: Responsive Layout Integrity**
    - **Validates: Requirements 8.1**

- [x] 6. Final Checkpoint
  - [x] Ensure all sections render correctly ✅
  - [x] Verify pricing displays accurate values ✅
  - [x] Confirm mobile responsiveness ✅
  - [x] Run Lighthouse audit (target 90+) ✅
  - [x] Site builds successfully with `npm run build` ✅
  - [x] Deployed to Vercel ✅

## Success Metrics

### Build Status
- ✅ **Clean build** - Zero errors, zero warnings
- ✅ **Bundle size** - Optimized with Vite
- ✅ **Deployment** - Live on Vercel

### Implementation Complete
- ✅ All 6 sections implemented (Hero, Problems, Solutions, Architecture, Pricing, Footer)
- ✅ All 4 Svelte islands working (MemoryHierarchy, ArchitectureDiagram, MobileNav, CodeBlock)
- ✅ Responsive design (320px - 2560px)
- ✅ SynthBrute aesthetic fully implemented
- ✅ Property tests passing
- ✅ Integration tests passing

**caliber-landing-page is production-ready and deployed!** 🚀

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Svelte components use `client:visible` for lazy hydration
- Design system tokens in `synthbrute.css` are the single source of truth for colors/spacing
