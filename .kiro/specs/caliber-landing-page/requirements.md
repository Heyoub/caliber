# Requirements Document

## Introduction

Landing page for CALIBER — a Postgres-native memory framework for AI agents. The page serves dual purposes: (1) showcase the open-source project for developers and potential contributors, and (2) present the commercial CALIBER Cloud offering with pricing. The visual direction is "SynthBrute" — a fusion of Neo-Brutalist structure with Synthwave/Vaporwave aesthetics and LiquidGlass effects.

## Glossary

- **Landing_Page**: The single-page marketing site for CALIBER
- **Hero_Section**: The above-the-fold primary visual and messaging area
- **Memory_Hierarchy_Visualization**: Interactive/animated diagram showing Trajectory → Scope → Artifact → Note
- **Pricing_Section**: Commercial offering display with tier information
- **SynthBrute_Aesthetic**: Design fusion of Neo-Brutalist grid structure with Synthwave neon and glass effects
- **CTA**: Call-to-action button or link

## Requirements

### Requirement 1: Hero Section with Memory Hierarchy Visualization

**User Story:** As a visitor, I want to immediately understand what CALIBER does through a visual representation of its core concept, so that I can quickly assess if it solves my problem.

#### Acceptance Criteria

1. WHEN the page loads, THE Hero_Section SHALL display the Memory_Hierarchy_Visualization as the centerpiece
2. THE Memory_Hierarchy_Visualization SHALL animate to show the relationship between Trajectory, Scope, Artifact, and Note entities
3. THE Hero_Section SHALL include a headline that communicates the core value proposition ("AI agents forget everything. CALIBER fixes that.")
4. THE Hero_Section SHALL include primary CTA buttons for GitHub and CALIBER Cloud
5. WHILE the user scrolls, THE Memory_Hierarchy_Visualization SHALL respond with parallax or depth effects

### Requirement 2: Problem Statement Section

**User Story:** As a developer evaluating CALIBER, I want to see the specific problems it solves, so that I can determine if my use case is addressed.

#### Acceptance Criteria

1. THE Landing_Page SHALL display a section outlining the 6 core problems: Context Amnesia, Hallucination, Multi-Agent Chaos, Token Waste, Hard-Coded Everything, No Audit Trail
2. WHEN displaying each problem, THE Landing_Page SHALL use brutalist card containers with neon accent glows
3. THE problem cards SHALL use sharp edges (0-2px border radius) with glass panel overlays

### Requirement 3: Solution/Features Section

**User Story:** As a developer, I want to understand how CALIBER solves each problem, so that I can evaluate the technical approach.

#### Acceptance Criteria

1. THE Landing_Page SHALL display solutions corresponding to each problem with code snippets
2. THE code snippets SHALL use syntax highlighting consistent with the SynthBrute aesthetic
3. WHEN displaying architecture information, THE Landing_Page SHALL show the ECS pattern and crate structure
4. THE solution section SHALL include the key stats: 10 crates, 165 tests, 57 property tests, 0 hard-coded defaults

### Requirement 4: Pricing Section

**User Story:** As a potential customer, I want to see clear pricing for CALIBER Cloud, so that I can budget for the service.

#### Acceptance Criteria

1. THE Pricing_Section SHALL display storage pricing: $1/GB/month, $10/GB/year (2 months free)
2. THE Pricing_Section SHALL display hot cache pricing: $0.15/MB/month
3. THE Pricing_Section SHALL indicate unlimited agents
4. THE Pricing_Section SHALL display trial information: 14 days, no credit card required
5. THE Pricing_Section SHALL include a CTA to start trial
6. THE Pricing_Section SHALL clarify that self-hosting is free (AGPL-3.0)

### Requirement 5: Visual Design System (SynthBrute Aesthetic)

**User Story:** As a visitor, I want a visually distinctive experience that reflects the project's personality, so that CALIBER stands out from generic AI/SaaS sites.

#### Acceptance Criteria

1. THE Landing_Page SHALL use a Neo-Brutalist grid structure with visible gutters and bounding boxes
2. THE Landing_Page SHALL apply Synthwave color palette: muted pink, purple, cyan with industrial rust accents
3. THE Landing_Page SHALL use glass panels (backdrop-blur, semi-transparent) that intentionally bleed outside brutalist frames
4. THE Landing_Page SHALL use typography: Space Grotesk or similar grotesque for titles, Inter for body
5. THE Landing_Page SHALL include neon glow effects on interactive elements and accents
6. WHILE elements enter viewport, THE Landing_Page SHALL animate with brutalist snap-in for structure and spring motion for glass elements

### Requirement 6: Technical Implementation

**User Story:** As a developer maintaining the site, I want a performant, modern stack, so that the site loads fast and is easy to update.

#### Acceptance Criteria

1. THE Landing_Page SHALL be built with Astro as the primary framework
2. THE Landing_Page SHALL use Svelte components for interactive elements
3. THE Landing_Page SHALL use TailwindCSS for styling
4. THE Landing_Page SHALL use MotionOne for Svelte animations
5. THE Landing_Page SHALL deploy to Vercel
6. THE Landing_Page SHALL score 90+ on Lighthouse performance

### Requirement 7: Navigation and Footer

**User Story:** As a visitor, I want clear navigation to key sections and external resources, so that I can find what I need.

#### Acceptance Criteria

1. THE Landing_Page SHALL include a fixed navigation bar with links to: Problem, Solution, Architecture, Pricing, GitHub
2. THE navigation bar SHALL use glass effect with backdrop blur
3. THE footer SHALL include links to: heyoub.dev, GitHub repository, documentation
4. THE footer SHALL display the AGPL-3.0 license notice

### Requirement 8: Responsive Design

**User Story:** As a mobile user, I want the landing page to work well on my device, so that I can evaluate CALIBER anywhere.

#### Acceptance Criteria

1. THE Landing_Page SHALL be fully responsive from 320px to 2560px viewport widths
2. THE Memory_Hierarchy_Visualization SHALL adapt to mobile with simplified animation
3. THE navigation SHALL collapse to a mobile menu on viewports below 768px
4. THE pricing cards SHALL stack vertically on mobile
