# Vue to Svelte Visual Fidelity Audit Report

**Date:** 2026-01-31
**Auditor:** Claude Code
**Scope:** 8 Vue reference files vs Svelte UI components in `/home/user/caliber/packages/ui/src/`

---

## 1. Accordion.vue

**Status:** ❌ NOT IMPLEMENTED

### Vue Reference Details

**Visual Specifications:**
- Border: `border-surface/40` (40% opacity)
- Border radius: `rounded-lg`
- Background: `bg-transparent` with `glass-card` class
- Shadows (3 states):
  - Default: `inset 0 -1px 2px rgba(30,41,59,0.4)`
  - Hover: `inset 0 -1px 4px rgba(30,41,59,0.5)`
  - Open: `inset 0 -2px 8px rgba(30,41,59,0.6)`
- Width: `w-[90%] mx-auto`
- Margin bottom: `mb-2`

**Animations:**
- Chevron rotation: `rotate-180` with `duration-200 ease-out`
- Content expand:
  - Max height: `0` to `1000px`
  - Opacity: `0` to `1`
  - Transform: `translateY(-2px)` to `translateY(0)`
  - Duration: `150ms ease-in-out`

**Unique Features:**
- Click-outside-to-close functionality
- Preview text truncation: Shows preview text when closed, full title when open
- **Inverted Triangle Text Layout:**
  ```css
  .inverted-triangle p:first-of-type { max-width: 75%; }
  .inverted-triangle p:nth-of-type(2) { max-width: 85%; }
  .inverted-triangle p:nth-of-type(3) { max-width: 90%; }
  .inverted-triangle p:nth-of-type(n+4) { max-width: 95%; }
  ```
- Text alignment: `text-align: justify`
- Line height: `1.6`

**Colors:**
- Primary text: `text-primary/60` (chevron) → `text-primary` on hover
- Content text: `text-gray-300` and `text-gray-300/90`
- Button padding: `py-1.5 px-2`

### What's Missing in Svelte

**No Accordion Component Found.** The closest alternatives are:
- `Dropdown.svelte` - Has open/close but no accordion-specific styling
- `Modal.svelte` - Different use case
- `Card.svelte` - No expand/collapse

### Code to Add

Create `/home/user/caliber/packages/ui/src/molecules/Accordion.svelte`:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { onMount, onDestroy } from 'svelte';

  interface Props {
    title: string;
    preview?: string;
    open?: boolean;
    class?: string;
    children: Snippet;
    titleSnippet?: Snippet;
  }

  let {
    title,
    preview,
    open = $bindable(false),
    class: className = '',
    children,
    titleSnippet
  }: Props = $props();

  let accordionRef: HTMLDivElement | undefined;

  function handleClickOutside(event: MouseEvent) {
    if (accordionRef && !accordionRef.contains(event.target as Node)) {
      open = false;
    }
  }

  onMount(() => {
    document.addEventListener('click', handleClickOutside);
  });

  onDestroy(() => {
    document.removeEventListener('click', handleClickOutside);
  });

  const contentHeight = $derived(open ? '1000px' : '0');
  const contentOpacity = $derived(open ? '1' : '0');
  const contentTransform = $derived(open ? 'translateY(0)' : 'translateY(-2px)');
</script>

<div class="mb-2 w-[90%] mx-auto" bind:this={accordionRef}>
  <div
    class={`
      rounded-lg bg-transparent border border-slate-600/40
      transition-all duration-200 ease-out overflow-hidden
      backdrop-blur-sm bg-slate-900/5
      ${!open ? 'shadow-[inset_0_-1px_2px_rgba(30,41,59,0.4)] hover:shadow-[inset_0_-1px_4px_rgba(30,41,59,0.5)]' : 'shadow-[inset_0_-2px_8px_rgba(30,41,59,0.6)]'}
      ${className}
    `}
  >
    <button
      onclick={(e) => { e.stopPropagation(); open = !open; }}
      class="w-full text-left py-1.5 px-2 relative group"
    >
      <div class="flex items-center justify-between">
        <div class="flex-1 pr-4">
          {#if preview && !open}
            <div class="text-base text-slate-300/90 leading-relaxed pr-8">
              {preview.split(' and innovate')[0] + ' and...'}
            </div>
          {:else if titleSnippet}
            {@render titleSnippet()}
          {:else}
            <div class="text-base text-slate-300 leading-relaxed">
              {title}
            </div>
          {/if}
        </div>
        <div
          class={`w-5 h-5 flex items-center justify-center transition-transform duration-200 ease-out ${open ? 'rotate-180' : ''}`}
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            class="w-4 h-4 text-teal-400/60 group-hover:text-teal-400 transition-colors duration-200"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M19 9l-7 7-7-7"
            />
          </svg>
        </div>
      </div>
    </button>
    <div
      class="transition-all duration-150 ease-in-out"
      style="max-height: {contentHeight}; opacity: {contentOpacity}; transform: {contentTransform};"
    >
      <div class="px-2 pb-2">
        <div class="text-base text-slate-300 leading-relaxed inverted-triangle">
          {@render children()}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .inverted-triangle {
    text-align: justify;
    max-width: 95%;
    margin: 0 auto;
    padding: 0.25em 0;
  }

  .inverted-triangle :global(p) {
    margin: 0;
    padding: 0.25em 0;
    line-height: 1.6;
  }

  .inverted-triangle :global(p:first-of-type) {
    max-width: 75%;
    margin: 0 auto;
  }

  .inverted-triangle :global(p:nth-of-type(2)) {
    max-width: 85%;
    margin: 0 auto;
  }

  .inverted-triangle :global(p:nth-of-type(3)) {
    max-width: 90%;
    margin: 0 auto;
  }

  .inverted-triangle :global(p:nth-of-type(n+4)) {
    max-width: 95%;
    margin: 0 auto;
  }
</style>
```

---

## 2. AdminPanel.vue

**Status:** ⚠️ PARTIALLY IMPLEMENTED (Tabs.svelte exists but layout pattern missing)

### Vue Reference Details

**Visual Specifications:**
- Layout: `flex flex-col md:flex-row h-full`
- Sidebar width: `w-full md:w-64`
- Border: `border-r border-gray-200 dark:border-gray-700 border-solid`
- Sidebar padding: `p-4`
- Gap between buttons: `gap-2`
- Title: `text-xl font-semibold mb-4`

**Button Styling (Inactive):**
- Background: `bg-white dark:bg-gray-800`
- Text: `text-gray-700 dark:text-gray-300`
- Border: `border border-gray-300 dark:border-gray-600`
- Hover: `hover:bg-gray-100 dark:hover:bg-gray-700`

**Button Styling (Active):**
- Background: `bg-indigo-600`
- Text: `text-white`
- Shadow: `shadow-md`
- Hover: `hover:bg-indigo-700`

**Shared Button Styles:**
- Padding: `px-4 py-2`
- Border radius: `rounded-md`
- Transition: `transition-colors duration-150 ease-in-out`
- Focus ring: `focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 dark:focus:ring-offset-gray-900`

**Content Area:**
- Flex: `flex-1`
- Scroll: `overflow-auto`
- Padding: `p-6`

### Comparison with Tabs.svelte

**What Tabs.svelte Has:**
- ✅ Active indicator animation
- ✅ Size variants (sm, md, lg)
- ✅ Color theming
- ✅ Keyboard navigation
- ✅ Badge support
- ✅ Vertical/horizontal orientation

**What's Different:**
- ❌ No sidebar layout pattern
- ❌ Different color scheme (teal vs indigo-600)
- ❌ No dark mode toggle built-in
- ❌ No content area slot with component rendering

### Code to Add

Add to `/home/user/caliber/packages/ui/src/organisms/AdminPanel.svelte`:

```svelte
<script lang="ts">
  import type { Snippet, Component } from 'svelte';
  import type { ColorPalette } from '../types';

  interface TabConfig {
    id: string;
    label: string;
    component?: Component;
    content?: Snippet;
  }

  interface Props {
    tabs: TabConfig[];
    activeTab?: string;
    title?: string;
    darkMode?: boolean;
    class?: string;
    ontabchange?: (tabId: string) => void;
  }

  let {
    tabs,
    activeTab = $bindable(tabs[0]?.id ?? ''),
    title = 'Admin Panel',
    darkMode = false,
    class: className = '',
    ontabchange
  }: Props = $props();

  function handleTabChange(tabId: string) {
    activeTab = tabId;
    ontabchange?.(tabId);
  }

  const activeTabConfig = $derived(tabs.find(t => t.id === activeTab));
</script>

<div class={`flex flex-col md:flex-row h-full ${darkMode ? 'bg-slate-900 text-slate-100' : 'bg-slate-50 text-slate-900'} ${className}`}>
  <!-- Sidebar Navigation -->
  <div class={`w-full md:w-64 flex-shrink-0 p-4 flex flex-col gap-2 border-r ${darkMode ? 'border-slate-700' : 'border-slate-200'}`}>
    <h2 class={`text-xl font-semibold mb-4 ${darkMode ? 'text-slate-200' : 'text-slate-800'}`}>
      {title}
    </h2>
    {#each tabs as tab (tab.id)}
      <button
        onclick={() => handleTabChange(tab.id)}
        class={`
          w-full text-left px-4 py-2 rounded-md transition-colors duration-150 ease-in-out
          focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500
          ${darkMode ? 'focus:ring-offset-slate-900' : ''}
          ${activeTab === tab.id
            ? 'bg-indigo-600 text-white shadow-md hover:bg-indigo-700'
            : darkMode
              ? 'bg-slate-800 text-slate-300 hover:bg-slate-700 border border-slate-600'
              : 'bg-white text-slate-700 hover:bg-slate-100 border border-slate-300'
          }
        `}
      >
        {tab.label}
      </button>
    {/each}
  </div>

  <!-- Content Area -->
  <div class="flex-1 overflow-auto p-6">
    {#if activeTabConfig?.content}
      {@render activeTabConfig.content()}
    {:else if activeTabConfig?.component}
      <svelte:component this={activeTabConfig.component} />
    {:else}
      <p class="text-slate-500">No content available</p>
    {/if}
  </div>
</div>
```

---

## 3. AiMainLayout.vue

**Status:** ⚠️ SIMPLE LAYOUT - No direct component needed (just CSS)

### Vue Reference Details

**Visual Specifications:**
- Container: `flex h-screen w-screen`
- Background: `bg-slate-950`
- Text color: `text-slate-100`
- Main content: `flex-1 overflow-y-auto bg-slate-900`

**Structure:**
```html
<div class="flex h-screen w-screen bg-slate-950 text-slate-100">
  <slot name="sidebar" />
  <main class="flex-1 overflow-y-auto bg-slate-900">
    <RouterView />
  </main>
  <slot name="context" v-if="showContext" />
</div>
```

### Analysis

This is a basic flexbox layout. No specific component needed - can be implemented directly in app layouts using Svelte's slot system.

**Missing from Svelte:** Nothing - this is standard layout CSS.

---

## 4. AnimatedButton.vue

**Status:** ⚠️ PARTIALLY IMPLEMENTED (Button.svelte exists but missing shine animation)

### Vue Reference Details

**Visual Specifications:**

**Base Classes:**
- `relative inline-flex items-center justify-center`
- `rounded-lg font-medium`
- `transition-all duration-200`
- `overflow-hidden`

**Size Classes:**
```typescript
sm: 'text-xs px-3 py-1.5'
md: 'text-sm px-4 py-2'
lg: 'text-base px-6 py-3'
```

**Variant: Default**
- Background: `bg-white/10`
- Text: `text-white`
- Border: `border border-white/20`
- Hover: `hover:bg-white/20`

**Variant: Outline**
- Background: `bg-transparent`
- Text: `text-white`
- Border: `border border-white/20`
- Hover: `hover:bg-white/10`

**Variant: ForgeStack**
- Text: `text-white`
- Border: `border-0`
- **Gradient layers:**
  ```css
  /* Base gradient */
  background: linear-gradient(to right,
    from-forgestack-teal/80 to-forgestack-purple/80
  );
  opacity: 0.6;

  /* Hover gradient */
  background: linear-gradient(to right,
    from-forgestack-teal to-forgestack-purple
  );
  opacity: 0 → 0.9 on hover;
  transition: opacity 300ms;
  ```

**Animations:**

**Shine Effect** (CRITICAL - MISSING IN SVELTE):
```css
@keyframes shine {
  from { left: -100%; }
  to { left: 200%; }
}

button:hover::after {
  content: "";
  position: absolute;
  top: 0;
  left: -100%;
  width: 50%;
  height: 100%;
  background: linear-gradient(
    to right,
    transparent,
    rgba(255, 255, 255, 0.2),
    transparent
  );
  transform: skewX(-25deg);
  animation: shine 1.5s infinite;
}
```

**Other Features:**
- Tap highlight: `-webkit-tap-highlight-color: transparent`
- Content z-index: `relative z-10 flex items-center gap-2`

### Comparison with Button.svelte

**What Button.svelte Has:**
- ✅ Multiple color themes (teal, coral, purple, pink, mint, amber, slate, ghost)
- ✅ Size variants
- ✅ Glow effects
- ✅ Glass morphism
- ✅ Hover effects (lift, glow, scale, brighten, border)
- ✅ Press effects (sink, scale, darken)
- ✅ Loading state
- ✅ Gradient backgrounds
- ✅ Border highlights
- ✅ Outer glow effect
- ✅ Hamburger menu variant

**What's Missing:**
- ❌ **Shine animation** (sweeping light effect)
- ❌ ForgeStack-specific teal/purple gradient variant
- ❌ Simple default/outline/forgestack variants matching Vue
- ❌ White/20 opacity borders for light overlay buttons

### Code to Add

Add to `/home/user/caliber/packages/ui/src/atoms/Button.svelte` (in style section):

```css
/* Add after existing animations */

/* Shine effect for all buttons on hover */
button:hover::before,
a:hover::before {
  content: "";
  position: absolute;
  top: 0;
  left: -100%;
  width: 50%;
  height: 100%;
  background: linear-gradient(
    to right,
    transparent,
    rgba(255, 255, 255, 0.2),
    transparent
  );
  transform: skewX(-25deg);
  animation: shine 1.5s infinite;
  z-index: 8;
  pointer-events: none;
}

@keyframes shine {
  from {
    left: -100%;
  }
  to {
    left: 200%;
  }
}

/* Disable tap highlight on mobile */
button, a {
  -webkit-tap-highlight-color: transparent;
}
```

Add ForgeStack color config to the `colorConfigs` object:

```typescript
forgestack: {
  bg: 'bg-transparent',
  hover: 'hover:bg-transparent',
  text: 'text-white',
  border: 'border-0',
  glow: 'transparent',
  outerGlow: 'transparent',
  gradient: 'from-[hsl(176,55%,45%,0.8)] to-[hsl(270,55%,52%,0.8)]',
  pressGradient: 'from-[hsl(176,55%,45%)] to-[hsl(270,55%,52%)]',
},
```

---

## 5. App.vue

**Status:** ✅ NOT APPLICABLE (Just imports and routing)

### Vue Reference Details

This file only contains:
- Font imports (`@fontsource/space-grotesk`)
- CSS imports (fonts.css, scroll.css, global.css, matter.css, design-system.css)
- PathProvider wrapper
- Router view

**Analysis:** This is application-level configuration, not a reusable component. No Svelte equivalent needed.

---

## 6. AuthLayout.vue

**Status:** ❌ COMPLEX LAYOUT - Not implemented in Svelte

### Vue Reference Details

**Visual Specifications:**

**Root Container:**
- Width: `w-full min-h-screen`
- Background: `bg-emi-bg-dark` (custom color)
- Text: `text-slate-100`
- Overflow: `overflow-x-hidden`

**Mobile Menu Button:**
- Position: `fixed top-9 left-7 z-[60]`
- Size: `w-[3rem] h-[3rem]`
- Background: `bg-emi-primary/20 hover:bg-emi-primary/30`
- Border: `border border-transparent hover:border-emi-primary/50`
- Border radius: `rounded-lg`
- Shadow (default): `0 0.25rem 0.75rem rgba(79,209,197,0.25)`
- Shadow (hover): `0 0.5rem 1.5rem rgba(79,209,197,0.35)`
- Transition: `transition-all duration-300`

**Gradient Glow Effect:**
```css
background: linear-gradient(to right,
  from-emi-primary/0 via-emi-primary/10 to-emi-primary/0
);
opacity: 0 → 1 on hover;
transition: opacity 500ms;
blur: xl;
```

**Hamburger Lines:**
- Width: `w-5` (20px / 1.25rem)
- Height: `h-[0.125rem]` (2px)
- Background: `bg-slate-100`
- Transition: `transition-transform duration-300 origin-center`
- Transform (open):
  - Line 1: `rotate(45deg) translateY(0.375rem)` (6px)
  - Line 2: `rotate(-45deg) translateY(-0.375rem)` (-6px)

**Mobile Menu Overlay:**
- Background: `bg-emi-bg-dark/95 backdrop-blur-md`
- Z-index: `z-50`
- Transition: `transition-opacity duration-300`
- Opacity: `0` → `1` when open
- Pointer events: `none` → `auto` when open

**Main Layout:**
- Display: `flex h-screen overflow-hidden`
- Sidebar width: `w-[20vw] min-w-[20vw] max-w-[20vw]`
- Sidebar background: `bg-emi-bg-dark`

**Content Container:**
- Flex: `flex-1`
- Background: `bg-emi-bg-dark`
- Padding: `pt-6 px-4 md:pr-6 md:pl-0`

**Content Inner Box:**
- Background: `bg-emi-bg-dark/90 backdrop-blur-xl`
- Border: `border border-emi-primary/30`
- Border radius (not auth): `rounded-tl-[1.875rem] rounded-tr-[1.875rem]` (30px top corners)
- Border radius (auth): `rounded-lg`
- Shadow: `shadow-xl`

### What's Missing in Svelte

**No AuthLayout component.** The closest is Sidebar.svelte, but it lacks:
- ❌ Mobile hamburger menu with animated lines
- ❌ Mobile overlay navigation
- ❌ Fixed positioning logic for menu button
- ❌ Route-specific conditional rendering (hiding sidebar on /auth)
- ❌ Specific emi-primary color theming
- ❌ Large rounded top corners (1.875rem)
- ❌ Body scroll lock on mobile menu open

### Code to Add

Create `/home/user/caliber/packages/ui/src/organisms/AuthLayout.svelte`:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    showSidebar?: boolean;
    sidebarWidth?: string;
    mobileMenuOpen?: boolean;
    children: Snippet;
    sidebar?: Snippet;
    mobileMenu?: Snippet;
    onToggleMobileMenu?: () => void;
  }

  let {
    showSidebar = true,
    sidebarWidth = '20vw',
    mobileMenuOpen = $bindable(false),
    children,
    sidebar,
    mobileMenu,
    onToggleMobileMenu
  }: Props = $props();

  function toggleMobileMenu() {
    mobileMenuOpen = !mobileMenuOpen;
    if (typeof document !== 'undefined') {
      document.body.classList.toggle('mobile-menu-active', mobileMenuOpen);
    }
    onToggleMobileMenu?.();
  }

  function closeMobileMenu() {
    mobileMenuOpen = false;
    if (typeof document !== 'undefined') {
      document.body.classList.remove('mobile-menu-active');
    }
  }

  const hamburgerLine1Style = $derived(
    mobileMenuOpen
      ? 'transform: rotate(45deg) translateY(0.375rem);'
      : 'transform: none;'
  );

  const hamburgerLine2Style = $derived(
    mobileMenuOpen
      ? 'transform: rotate(-45deg) translateY(-0.375rem);'
      : 'transform: none;'
  );
</script>

<div class="w-full min-h-screen bg-slate-900 text-slate-100 overflow-x-hidden">
  <!-- Mobile menu button -->
  {#if showSidebar}
    <div class="md:hidden fixed top-9 left-7 z-[60]">
      <button
        onclick={toggleMobileMenu}
        class="
          w-[3rem] h-[3rem] flex flex-col items-center justify-center gap-[0.5rem]
          bg-teal-500/20 hover:bg-teal-500/30
          border border-transparent hover:border-teal-500/50
          rounded-lg
          shadow-[0_0.25rem_0.75rem_rgba(79,209,197,0.25)] hover:shadow-[0_0.5rem_1.5rem_rgba(79,209,197,0.35)]
          transition-all duration-300 group relative overflow-hidden
        "
        aria-expanded={mobileMenuOpen}
        aria-haspopup="true"
      >
        <!-- Gradient glow -->
        <div class="absolute inset-0 bg-gradient-to-r from-teal-500/0 via-teal-500/10 to-teal-500/0 opacity-0 group-hover:opacity-100 transition-opacity duration-500 blur-xl pointer-events-none"></div>

        <!-- Hamburger lines -->
        <span
          class="block w-5 h-[0.125rem] bg-slate-100 transition-transform duration-300 origin-center"
          style={hamburgerLine1Style}
        ></span>
        <span
          class="block w-5 h-[0.125rem] bg-slate-100 transition-transform duration-300 origin-center"
          style={hamburgerLine2Style}
        ></span>

        <!-- Border highlight -->
        <div class="absolute inset-0 bg-gradient-to-b from-white/0 via-slate-100/5 to-white/0 opacity-0 group-hover:opacity-100 transition-opacity"></div>
        <div class="absolute inset-0 border border-teal-500/30 rounded-lg opacity-0 group-hover:opacity-100 transition-opacity scale-[1.02]"></div>
      </button>
    </div>
  {/if}

  <!-- Mobile menu overlay -->
  {#if showSidebar}
    <div
      class={`
        md:hidden fixed inset-0 bg-slate-900/95 backdrop-blur-md z-50 transition-opacity duration-300
        ${mobileMenuOpen ? 'opacity-100 pointer-events-auto' : 'opacity-0 pointer-events-none'}
      `}
    >
      {#if mobileMenu}
        {@render mobileMenu()}
      {:else}
        <nav class="flex flex-col items-center justify-center h-full px-6">
          <p class="text-slate-400">Mobile menu content</p>
        </nav>
      {/if}
    </div>
  {/if}

  <!-- Main layout -->
  <div class={`flex h-screen overflow-hidden ${mobileMenuOpen ? 'mobile-menu-active' : ''}`}>
    <!-- Sidebar -->
    {#if showSidebar && sidebar}
      <aside
        class="flex-shrink-0 bg-slate-900 scrollbar-none hidden md:block"
        style="width: {sidebarWidth}; min-width: {sidebarWidth}; max-width: {sidebarWidth};"
      >
        {@render sidebar()}
      </aside>
    {/if}

    <!-- Content wrapper -->
    <div class="flex-1 bg-slate-900 flex flex-col h-screen overflow-hidden relative">
      <div class="absolute inset-0 bg-slate-900 pointer-events-none"></div>

      <!-- Content area -->
      <div class="flex-1 bg-slate-900 overflow-hidden pt-6 px-4 md:pr-6 md:pl-0 relative z-10">
        <div
          class={`
            w-full h-full overflow-hidden shadow-xl
            bg-slate-900/90 backdrop-blur-xl
            border border-teal-500/30
            ${showSidebar ? 'rounded-tl-[1.875rem] rounded-tr-[1.875rem]' : 'rounded-lg'}
          `}
        >
          <div class="h-full">
            {@render children()}
          </div>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  :global(body.mobile-menu-active) {
    overflow: hidden;
  }
</style>
```

---

## 7. AuthNav.vue

**Status:** ⚠️ COMPLEX COMPONENT - Partially covered by Sidebar + NestedMenu + Dropdown

### Vue Reference Details

**Visual Specifications:**

**Root Nav:**
- Background: `bg-emi-bg-dark`
- Padding: `px-2 pt-[1.65rem]`
- Position: `relative z-50`
- Text: `text-slate-100`

**ForgeStack Button:**
- Color: `glassy` (custom color variant)
- Size: `lg`
- Width: `w-full`
- Margin: `mb-1`
- Content: Hamburger icon + Logo image + "orgeStack" text

**Hamburger Icon (in button):**
- Container: `h-8 w-3 flex flex-col ml-4 mt-[1rem]`
- Lines: `w-[1.875rem] h-[0.33rem] bg-slate-100/90 rounded-full`
- Gap: `gap-[0.5rem]`
- Transition: `transition-all duration-300 origin-center`

**Logo:**
- Source: `/IMG/AXLG.svg`
- Size: `h-14 w-auto`
- Opacity: `opacity-90`

**Text:**
- Margin: `mt-[1rem] ml-[-1.75rem]`
- Font: `text-3xl font-semibold`
- Color: `text-white`
- Tracking: `tracking-wider`

**Dropdown Menu:**
- Position: Uses `@floating-ui/vue` with offset 25px
- Placement: `right-start`
- Width: `w-[10rem]`
- Background: `bg-[#1a2e35e6] backdrop-blur-xl`
- Border: `border border-[#4fd1c54d]` (teal with opacity)
- Border radius: `rounded-lg`
- Shadow: `0 0.25rem 0.25rem rgba(79,209,197,0.15)`
- Transition: `transition-all duration-300`
- Transform closed: `opacity-0 translate-y-2 pointer-events-none`
- Transform open: `opacity-1 translate-y-0 pointer-events-auto`

**Box Shadow (Dropdown):**
```css
box-shadow:
  0 0 1px 1px rgba(41, 171, 226, 0.1),
  0 0 1px 1px rgba(23, 32, 51, 0.5),
  0 0 5px 1px rgba(166, 109, 196, 0.1);

/* Hover */
box-shadow:
  0 0 5px 1px rgba(41, 171, 226, 0.1),
  0 0 5px 1px rgba(252, 88, 81, 0.1),
  0 0 5px 1px rgba(70, 214, 172, 0.1);
```

**Laser Effect:**
- Position: `absolute inset-0 pointer-events-none`
- Opacity: `opacity-10` → `opacity-20` on hover
- Mix blend: `mix-blend-plus-lighter`
- Transform: `transform-gpu`
- Transition: `transition-opacity duration-500`

**Laser Gradient (on item hover):**
```css
radial-gradient(
  circle at ${x}px ${y}px,
  rgba(79, 209, 197, 0.2) 0%,
  transparent 40%
)
```

**Laser Gradient (general hover):**
```css
radial-gradient(
  circle at ${x}px ${y}px,
  rgba(79, 209, 197, 0.1) 0%,
  transparent 40%
)
```

**Category Header:**
- Padding: `px-[1rem] pt-[0.75rem] pb-[0.25rem]`
- Font: `text-xs font-semibold`
- Color: `text-slate-300/80`
- Transform: `uppercase tracking-wider`
- Icon: `w-4 h-4 text-[#4FD1C5] opacity-80`

**HR Separator:**
- Border: `border-t border-[#4fd1c54d]`
- Margin: `my-1 mx-4`

**Link Items:**
- Padding: `px-[1rem] py-[0.5rem]`
- Font: `text-sm`
- Color: `text-slate-100/90 hover:text-white`
- Background: `transparent` → `bg-[#4FD1C5]/10` on hover
- Transition: `transition-all duration-300`

**Underline Animation:**
```css
position: absolute;
bottom: 0;
left: 1/2;
right: 1/2;
height: 1px;
background: #4FD1C5 (80% opacity);
transition: all 300ms;
shadow: 0 0.5rem 1.5rem rgba(79,209,197,0.5);

/* Hover */
left: 1rem;
right: 1rem;
```

**Middle Content Tabs Area:**
- Flex: `flex-grow flex flex-col`
- Overflow: `overflow-hidden`
- Margin: `mt-4 mb-4`
- Border: `border border-emi-primary/30`
- Border radius: `rounded-lg`
- Background: `bg-emi-bg-dark`

**Footer Section:**
- Margin: `mt-auto pt-4 px-2 pb-4`
- Card padding: `px-2 py-3`
- Card background: `bg-slate-800/50 backdrop-blur-sm`
- Card border: `border border-[#4fd1c54d]`
- Card radius: `rounded-lg`
- Title color: `text-[#4FD1C5]`
- Text: `text-xs text-slate-300/70`

### Comparison with Svelte Components

**Sidebar.svelte provides:**
- ✅ Navigation structure
- ✅ Laser effect on hover
- ✅ Collapsible sections
- ✅ Footer area
- ✅ Active states

**NestedMenu.svelte provides:**
- ✅ Category headers
- ✅ HR separators
- ✅ Underline animations
- ✅ Icon support

**Dropdown.svelte provides:**
- ✅ Floating positioning
- ✅ Laser effect
- ✅ Click outside to close
- ✅ Keyboard navigation

**What's Missing:**
- ❌ ForgeStack button with logo + hamburger combo
- ❌ Hamburger animation integrated with dropdown toggle
- ❌ Floating-UI with specific offset 25px and right-start placement
- ❌ Specific teal-themed borders (`#4fd1c54d`)
- ❌ Multi-layered box shadow (blue, dark, purple mix)
- ❌ Tab component slot system (EmiTabs, SettingsTabs, SupportTabs)
- ❌ Route-based tab rendering logic
- ❌ Hamburger lines with specific dimensions (w-[1.875rem] h-[0.33rem])

### Code to Add

**Option 1:** Enhance Dropdown.svelte with AuthNav-specific styling

**Option 2:** Create specialized `/home/user/caliber/packages/ui/src/organisms/AuthNav.svelte`

Due to complexity, recommend creating a dedicated component:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Dropdown from '../molecules/Dropdown.svelte';
  import Button from '../atoms/Button.svelte';

  interface NavLink {
    text: string;
    href: string;
    icon?: Snippet;
  }

  interface NavCategory {
    text: string;
    icon?: Snippet;
    children: NavLink[];
  }

  interface Props {
    categories: NavCategory[];
    logoSrc?: string;
    currentPath?: string;
    userName?: string;
    middleContent?: Snippet;
    footer?: Snippet;
    class?: string;
    onNavigate?: (href: string) => void;
  }

  let {
    categories,
    logoSrc = '/IMG/AXLG.svg',
    currentPath = '',
    userName = 'User',
    middleContent,
    footer,
    class: className = '',
    onNavigate
  }: Props = $props();

  let isDropdownOpen = $state(false);
  let buttonRef: HTMLButtonElement | undefined;
  let dropdownRef: HTMLDivElement | undefined;
  let line1Ref: HTMLSpanElement | undefined;
  let line2Ref: HTMLSpanElement | undefined;
  let laserEffectRef: HTMLDivElement | undefined;

  function toggleDropdown() {
    isDropdownOpen = !isDropdownOpen;

    if (line1Ref && line2Ref) {
      if (isDropdownOpen) {
        line1Ref.style.transform = 'rotate(45deg) translateY(0.375rem)';
        line2Ref.style.transform = 'rotate(-45deg) translateY(-0.375rem)';
      } else {
        line1Ref.style.transform = 'none';
        line2Ref.style.transform = 'none';
      }
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (!laserEffectRef || !dropdownRef) return;

    const rect = dropdownRef.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const linkElement = document.elementFromPoint(e.clientX, e.clientY) as HTMLElement;
    const linkTarget = linkElement?.closest('a');

    if (linkTarget) {
      laserEffectRef.style.background = `
        radial-gradient(
          circle at ${x}px ${y}px,
          rgba(79, 209, 197, 0.2) 0%,
          transparent 40%
        )
      `;
      laserEffectRef.style.opacity = '0.8';
    } else {
      laserEffectRef.style.background = `
        radial-gradient(
          circle at ${x}px ${y}px,
          rgba(79, 209, 197, 0.1) 0%,
          transparent 40%
        )
      `;
      laserEffectRef.style.opacity = '0.2';
    }
  }
</script>

<nav class={`flex flex-col bg-slate-900 min-h-screen px-2 pt-[1.65rem] relative z-50 text-slate-100 ${className}`}>
  <!-- ForgeStack Button -->
  <button
    bind:this={buttonRef}
    onclick={toggleDropdown}
    class="
      relative mb-1 w-full px-6 py-3 rounded-lg
      bg-slate-800/40 backdrop-blur-xl border border-slate-700/30
      hover:bg-slate-800/60 hover:border-teal-500/50
      transition-all duration-300 group overflow-hidden
    "
    aria-expanded={isDropdownOpen}
    aria-haspopup="true"
  >
    <span class="flex flex-row items-center justify-center gap-0">
      <!-- Hamburger Icon -->
      <span class="h-8 w-3 flex flex-col ml-4 mt-[1rem] items-center justify-center gap-[0.5rem]">
        <span
          bind:this={line1Ref}
          class="w-[1.875rem] h-[0.33rem] bg-slate-100/90 rounded-full transition-all duration-300 origin-center"
        ></span>
        <span
          bind:this={line2Ref}
          class="w-[1.875rem] h-[0.33rem] bg-slate-100/90 rounded-full transition-all duration-300 origin-center"
        ></span>
      </span>
      <!-- Logo -->
      <img src={logoSrc} alt="Logo" class="h-14 w-auto opacity-90" />
      <span class="mt-[1rem] ml-[-1.75rem] text-3xl font-semibold text-white tracking-wider">
        orgeStack
      </span>
    </span>
  </button>

  <!-- Dropdown Menu -->
  {#if isDropdownOpen}
    <div
      bind:this={dropdownRef}
      class="
        absolute mt-3 w-[10rem] h-auto
        bg-[#1a2e35e6] backdrop-blur-xl
        border border-[#4fd1c54d] rounded-lg
        shadow-[0_0.25rem_0.25rem_rgba(79,209,197,0.15)]
        overflow-hidden transition-all duration-300
        auth-nav-dropdown
      "
      style="top: 100%; left: calc(100% + 25px);"
      onmousemove={handleMouseMove}
    >
      <!-- Laser effect -->
      <div
        bind:this={laserEffectRef}
        class="absolute inset-0 pointer-events-none opacity-10 mix-blend-plus-lighter transform-gpu hover:opacity-20 transition-opacity duration-500"
      ></div>

      <!-- Links -->
      <div class="relative z-10 py-1">
        {#each categories as category, index (category.text)}
          <div class="category-block">
            {#if index > 0}
              <hr class="border-t border-[#4fd1c54d] my-1 mx-4" />
            {/if}

            <div class="flex items-center gap-2 px-[1rem] pt-[0.75rem] pb-[0.25rem] text-xs font-semibold text-slate-300/80 uppercase tracking-wider">
              {#if category.icon}
                <span class="w-4 h-4 text-[#4FD1C5] opacity-80">
                  {@render category.icon()}
                </span>
              {/if}
              <span>{category.text}</span>
            </div>

            {#each category.children as link (link.href)}
              <a
                href={link.href}
                class="
                  block px-[1rem] py-[0.5rem] text-slate-100/90 hover:text-white text-left
                  hover:bg-[#4FD1C5]/10 transition-all duration-300
                  relative group/link text-sm
                "
                aria-current={currentPath === link.href ? 'page' : undefined}
                onclick={(e) => { e.preventDefault(); onNavigate?.(link.href); }}
              >
                <span class="relative z-10">{link.text}</span>
                <span class="
                  absolute bottom-0 left-1/2 right-1/2 h-px bg-[#4FD1C5]/80
                  group-hover/link:left-[1rem] group-hover/link:right-[1rem]
                  transition-all duration-300
                  shadow-[0_0.5rem_1.5rem_rgba(79,209,197,0.5)]
                "></span>
              </a>
            {/each}
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Middle content area -->
  <div class="flex-grow flex flex-col overflow-hidden mt-4 mb-4 border border-teal-500/30 rounded-lg bg-slate-900">
    {#if middleContent}
      {@render middleContent()}
    {/if}
  </div>

  <!-- Footer -->
  <div class="mt-auto pt-4 px-2 pb-4 text-center">
    {#if footer}
      {@render footer()}
    {:else}
      <div class="px-2 py-3 rounded-lg bg-slate-800/50 backdrop-blur-sm border border-[#4fd1c54d]">
        <h3 class="text-xs font-semibold mb-2 text-[#4FD1C5] uppercase tracking-wider">
          Hi, {userName}
        </h3>
        <div class="text-xs text-slate-300/70 space-y-1">
          <div class="flex justify-center gap-3">
            <a href="/support" class="hover:text-[#4FD1C5] transition-colors">Support</a>
            <span>&bull;</span>
            <a href="/terms" class="hover:text-[#4FD1C5] transition-colors">Terms</a>
          </div>
        </div>
      </div>
    {/if}
  </div>
</nav>

<style>
  .auth-nav-dropdown {
    box-shadow:
      0 0 1px 1px rgba(41, 171, 226, 0.1),
      0 0 1px 1px rgba(23, 32, 51, 0.5),
      0 0 5px 1px rgba(166, 109, 196, 0.1);
  }

  .auth-nav-dropdown:hover {
    box-shadow:
      0 0 5px 1px rgba(41, 171, 226, 0.1),
      0 0 5px 1px rgba(252, 88, 81, 0.1),
      0 0 5px 1px rgba(70, 214, 172, 0.1);
  }
</style>
```

---

## 8. ChatBubble.vue

**Status:** ✅ WELL IMPLEMENTED (ChatMessage.svelte)

### Vue Reference Details

**Visual Specifications:**

**System Message:**
- Alignment: `text-center`
- Font: `text-sm italic`
- Color: `text-gray-400`
- Margin: `my-2`

**Message Container:**
- Padding: `p-3`
- Flex: `flex items-start`
- Justify: `justify-end` (user only)

**Avatar:**
- Size: `w-6 h-6`
- Border radius: `rounded-full`
- Flex: `flex items-center justify-center`
- Background: `bg-background/80 dark:bg-background/60`
- Shadow: `shadow-sm`
- Icon size: `w-3 h-3`
- Icon colors:
  - User: `text-orange-500 dark:text-orange-400`
  - Assistant: `text-purple-500 dark:text-purple-400`
- Margin:
  - User: `order-last ml-2`
  - Assistant: `mr-2`

**Message Content Area:**
- Padding: `p-3`
- Border radius: `rounded-xl`
- Shadow: `shadow-md`
- Background: `bg-slate-700/20`
- Backdrop: `backdrop-blur-lg`
- Border: `border border-slate-500/30`
- Text: `text-slate-100 dark:text-slate-50`

**Role Label:**
- Font: `text-xs font-medium mb-1.5`
- Colors:
  - User: `text-orange-400 dark:text-orange-300`
  - Assistant: `text-purple-400 dark:text-purple-300`

**Model Metadata:**
- Font: `text-[0.625rem]` (10px)
- Color: `text-slate-400`
- Opacity: `opacity-70`
- Margin: `mb-1`

**Typing Indicator Container:**
- Flex: `flex items-center gap-3`
- Padding: `p-1`

**Sparkles Icon Container:**
- Size: `w-6 h-6`
- Flex shrink: `flex-shrink-0`
- Border radius: `rounded-full`
- Background: `bg-gradient-to-br from-purple-500/30 to-orange-500/20`
- Icon: `w-3 h-3 text-purple-400`

**Dot Flashing Animation:**
```css
.dot-flashing {
  position: relative;
  width: 6px;
  height: 6px;
  border-radius: 5px;
  background-color: #9880ff;
  animation: dot-flashing 1s infinite linear alternate;
  animation-delay: 0.5s;
}

.dot-flashing::before {
  left: -10px;
  animation-delay: 0s;
}

.dot-flashing::after {
  left: 10px;
  animation-delay: 1s;
}

@keyframes dot-flashing {
  0% { background-color: #9880ff; }
  50%, 100% { background-color: rgba(152, 128, 255, 0.2); }
}
```

**Copy Button:**
- Position: `absolute top-2 right-2`
- Padding: `p-1`
- Border radius: `rounded`
- Hover: `hover:bg-slate-200/60 dark:hover:bg-slate-700/60`
- Transition: `transition`
- Z-index: `z-index: 2`
- Icon: `w-4 h-4`
- Icon color (default): `text-gray-400`
- Icon color (copying): `text-teal-500 animate-pulse`

**Message Content:**
- Font: `text-sm`
- Word break: `break-words`
- White space: `whitespace-pre-wrap`

**Markdown Styles:**
```css
.bubble h1, h2, h3, h4, h5, h6 {
  font-family: grotesk;
  font-weight: semibold;
}

.bubble p {
  margin: 0.5rem 0;
}

.bubble code {
  background-color: rgba(255,255,255,0.1);
  padding: 0.1rem 0.4rem;
  border-radius: 0.25rem;
  font-family: monospace;
}

.bubble pre {
  background-color: rgba(255,255,255,0.05);
  padding: 0.5rem;
  border-radius: 0.25rem;
  overflow-x: auto;
}
```

### Comparison with ChatMessage.svelte

**What ChatMessage.svelte Has:**
- ✅ System message centered styling
- ✅ User/assistant role differentiation
- ✅ Avatar with icons (different SVG paths)
- ✅ Glassmorphic bubble styling
- ✅ Role label with colors
- ✅ Model metadata display
- ✅ Typing/streaming indicator
- ✅ Sparkles icon
- ✅ Dot flashing animation
- ✅ Copy to clipboard button
- ✅ Timestamp formatting
- ✅ HTML escaping for user messages
- ✅ CMS content support
- ✅ Snippet-based slots

**What's Different:**
- Icon paths: Vue uses Lucide icons, Svelte uses inline SVG paths
- Color values:
  - Vue: `text-orange-500/400` (user), `text-purple-500/400` (assistant)
  - Svelte: `text-coral-400` (user), `text-purple-400` (assistant)
- Dot flashing color:
  - Vue: `#9880ff`
  - Svelte: `hsl(var(--purple-400))`
- Avatar size:
  - Vue: `w-6 h-6` (24px)
  - Svelte: `w-8 h-8` (32px)
- Copy button opacity:
  - Vue: Always visible
  - Svelte: `opacity-0 group-hover:opacity-100`

**Visual Fidelity:**
- Avatar is slightly larger in Svelte (8 vs 6)
- Orange → Coral color shift
- Copy button hidden by default (better UX in Svelte)

### Minor Tweaks Recommended

Adjust ChatMessage.svelte avatar size to match Vue:

```svelte
<!-- Change line 138 -->
<div class="w-6 h-6 rounded-full flex items-center justify-center bg-slate-800/80 shadow-sm border border-slate-700/50">
  <!-- ... -->
  <svg class="w-3 h-3 text-coral-400" ...>
```

And make copy button always visible:

```svelte
<!-- Change line 203 -->
<button
  onclick={handleCopy}
  title={...}
  class="absolute top-2 right-2 p-1 rounded hover:bg-slate-700/60 transition"
  class:animate-pulse={isCopying}
>
```

---

## Summary Table

| Vue File | Svelte Status | Visual Fidelity | Missing Features |
|----------|---------------|-----------------|------------------|
| Accordion.vue | ❌ Not Implemented | N/A | Entire component, inverted triangle layout |
| AdminPanel.vue | ⚠️ Partial (Tabs) | 70% | Sidebar layout, indigo theme, dark mode |
| AiMainLayout.vue | ✅ Not Needed | 100% | N/A (basic CSS) |
| AnimatedButton.vue | ⚠️ Partial (Button) | 85% | Shine animation, ForgeStack variant |
| App.vue | ✅ Not Applicable | N/A | N/A (app-level config) |
| AuthLayout.vue | ❌ Not Implemented | N/A | Mobile menu, hamburger, rounded corners |
| AuthNav.vue | ⚠️ Partial (Sidebar+Dropdown+NestedMenu) | 60% | ForgeStack button, multi-shadow, tab slots |
| ChatBubble.vue | ✅ Implemented (ChatMessage) | 95% | Minor: Avatar size, copy button visibility |

---

## Critical Missing Animations

### 1. Shine Animation (AnimatedButton.vue)
**Importance:** HIGH
**Visual Impact:** Creates premium feel with sweeping light effect

```css
@keyframes shine {
  from { left: -100%; }
  to { left: 200%; }
}
```

### 2. Accordion Expand/Collapse
**Importance:** HIGH
**Visual Impact:** Smooth height, opacity, and translateY transition

```css
transition: max-height 150ms ease-in-out,
            opacity 150ms ease-in-out,
            transform 150ms ease-in-out;
```

### 3. Hamburger Menu Line Rotation
**Importance:** MEDIUM
**Visual Impact:** Standard mobile menu interaction

```css
transform: rotate(45deg) translateY(0.375rem);
transform: rotate(-45deg) translateY(-0.375rem);
transition: transform 300ms;
```

### 4. Inverted Triangle Text Layout
**Importance:** MEDIUM
**Visual Impact:** Unique progressive text width pattern

```css
p:nth-of-type(1) { max-width: 75%; }
p:nth-of-type(2) { max-width: 85%; }
p:nth-of-type(3) { max-width: 90%; }
p:nth-of-type(n+4) { max-width: 95%; }
```

---

## Color Accuracy Notes

### Primary Colors (Teal/Emi-Primary)
- Vue: `#4FD1C5` → `rgba(79, 209, 197, x)`
- Svelte: `hsl(var(--teal-500))` → Should be `hsl(176, 55%, 45%)`

### Secondary Colors
- Purple: `#9880ff` (Vue) vs `hsl(var(--purple-400))` (Svelte)
- Orange/Coral: `orange-500` (Vue) vs `coral-400` (Svelte)

### Opacity Levels
- Vue commonly uses: `/10`, `/20`, `/30`, `/40`, `/50`, `/60`, `/80`, `/90`
- Svelte should match these exact values

---

## Recommendations

### Priority 1 (Must Have):
1. Create Accordion.svelte with full visual fidelity
2. Add shine animation to Button.svelte
3. Create AuthLayout.svelte with mobile menu

### Priority 2 (Should Have):
4. Create AdminPanel.svelte with sidebar layout
5. Enhance AuthNav.svelte with ForgeStack button pattern
6. Adjust ChatMessage.svelte avatar size to 6x6

### Priority 3 (Nice to Have):
7. Create ForgeStack color variant for Button
8. Add multi-shadow hover effects to dropdowns
9. Create tab slot system for dynamic content

---

**End of Audit Report**
