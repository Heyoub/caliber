/**
 * CALIBER App - Tailwind Configuration
 * Production-ready config with all design tokens
 */

/** @type {import('tailwindcss').Config} */
export default {
  content: [
    './src/**/*.{html,js,svelte,ts}',
    '../packages/ui/src/**/*.{svelte,ts}',
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // CALIBER brand palette
        teal: {
          50: 'hsl(175 70% 95%)',
          100: 'hsl(175 70% 88%)',
          200: 'hsl(175 70% 75%)',
          300: 'hsl(175 70% 60%)',
          400: 'hsl(175 70% 50%)',
          500: 'hsl(175 70% 40%)',
          600: 'hsl(175 70% 32%)',
          700: 'hsl(175 70% 25%)',
          800: 'hsl(175 70% 18%)',
          900: 'hsl(175 70% 12%)',
          950: 'hsl(175 70% 8%)',
        },
        coral: {
          50: 'hsl(15 85% 96%)',
          100: 'hsl(15 85% 90%)',
          200: 'hsl(15 85% 80%)',
          300: 'hsl(15 85% 68%)',
          400: 'hsl(15 85% 58%)',
          500: 'hsl(15 85% 50%)',
          600: 'hsl(15 85% 42%)',
          700: 'hsl(15 85% 34%)',
          800: 'hsl(15 85% 26%)',
          900: 'hsl(15 85% 18%)',
          950: 'hsl(15 85% 10%)',
        },
        mint: {
          50: 'hsl(165 70% 96%)',
          100: 'hsl(165 70% 90%)',
          200: 'hsl(165 70% 78%)',
          300: 'hsl(165 70% 62%)',
          400: 'hsl(165 70% 52%)',
          500: 'hsl(165 70% 45%)',
          600: 'hsl(165 70% 36%)',
          700: 'hsl(165 70% 28%)',
          800: 'hsl(165 70% 20%)',
          900: 'hsl(165 70% 14%)',
          950: 'hsl(165 70% 8%)',
        },
        lavender: {
          50: 'hsl(265 70% 97%)',
          100: 'hsl(265 70% 92%)',
          200: 'hsl(265 70% 82%)',
          300: 'hsl(265 70% 70%)',
          400: 'hsl(265 70% 62%)',
          500: 'hsl(265 70% 55%)',
          600: 'hsl(265 70% 46%)',
          700: 'hsl(265 70% 38%)',
          800: 'hsl(265 70% 28%)',
          900: 'hsl(265 70% 18%)',
          950: 'hsl(265 70% 10%)',
        },
        purple: {
          50: 'hsl(270 70% 97%)',
          100: 'hsl(270 70% 92%)',
          200: 'hsl(270 70% 82%)',
          300: 'hsl(270 70% 72%)',
          400: 'hsl(270 70% 65%)',
          500: 'hsl(270 70% 60%)',
          600: 'hsl(270 70% 50%)',
          700: 'hsl(270 70% 40%)',
          800: 'hsl(270 70% 30%)',
          900: 'hsl(270 70% 20%)',
          950: 'hsl(270 70% 12%)',
        },
        pink: {
          50: 'hsl(330 80% 97%)',
          100: 'hsl(330 80% 92%)',
          200: 'hsl(330 80% 82%)',
          300: 'hsl(330 80% 72%)',
          400: 'hsl(330 80% 62%)',
          500: 'hsl(330 80% 52%)',
          600: 'hsl(330 80% 44%)',
          700: 'hsl(330 80% 36%)',
          800: 'hsl(330 80% 26%)',
          900: 'hsl(330 80% 18%)',
          950: 'hsl(330 80% 10%)',
        },
        amber: {
          50: 'hsl(38 92% 96%)',
          100: 'hsl(38 92% 90%)',
          200: 'hsl(38 92% 78%)',
          300: 'hsl(38 92% 65%)',
          400: 'hsl(38 92% 55%)',
          500: 'hsl(38 92% 50%)',
          600: 'hsl(38 92% 42%)',
          700: 'hsl(38 92% 34%)',
          800: 'hsl(38 92% 26%)',
          900: 'hsl(38 92% 18%)',
          950: 'hsl(38 92% 10%)',
        },
        slate: {
          50: 'hsl(210 20% 98%)',
          100: 'hsl(214 20% 94%)',
          200: 'hsl(218 18% 86%)',
          300: 'hsl(220 16% 72%)',
          400: 'hsl(220 14% 54%)',
          500: 'hsl(220 14% 40%)',
          600: 'hsl(222 16% 30%)',
          700: 'hsl(222 18% 22%)',
          800: 'hsl(222 18% 20%)',
          900: 'hsl(225 20% 10%)',
          950: 'hsl(228 22% 6%)',
        },
        navy: {
          700: 'hsl(220 30% 18%)',
          800: 'hsl(220 30% 15%)',
          900: 'hsl(225 35% 10%)',
          950: 'hsl(228 40% 6%)',
        },
        // Surface and background aliases
        surface: {
          DEFAULT: 'hsl(225 20% 10%)',
          dark: 'hsl(228 22% 6%)',
          light: 'hsl(222 18% 20%)',
        },
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', '-apple-system', 'sans-serif'],
        mono: ['JetBrains Mono', 'Fira Code', 'monospace'],
        display: ['Space Grotesk', 'Inter', 'sans-serif'],
        title: ['Cal Sans', 'Space Grotesk', 'Inter', 'sans-serif'],
      },
      fontSize: {
        '2xs': ['0.625rem', { lineHeight: '0.875rem' }],
      },
      spacing: {
        18: '4.5rem',
        88: '22rem',
        128: '32rem',
      },
      borderRadius: {
        '4xl': '2rem',
        '5xl': '2.5rem',
      },
      backdropBlur: {
        xs: '2px',
      },
      boxShadow: {
        glow: '0 0 20px var(--tw-shadow-color, rgba(79, 209, 197, 0.25))',
        'glow-lg': '0 0 40px var(--tw-shadow-color, rgba(79, 209, 197, 0.35))',
        'glow-xl': '0 0 60px var(--tw-shadow-color, rgba(79, 209, 197, 0.45))',
        'inner-glow': 'inset 0 0 20px var(--tw-shadow-color, rgba(79, 209, 197, 0.15))',
      },
      animation: {
        // Blob/Lava animations
        'blob-move': 'blob-move 20s ease-in-out infinite',
        'blob-morph': 'blob-morph 8s ease-in-out infinite',
        'blob-float': 'blob-float 8s ease-in-out infinite',
        // Glow animations
        'pulse-glow': 'pulse-glow 4s ease-in-out infinite',
        'glow-pulse': 'glow-pulse 2s ease-in-out infinite',
        'text-glow': 'text-glow 2s ease-in-out infinite',
        // Gradient animations
        'gradient-flow': 'gradient-flow 15s ease infinite',
        'gradient-x': 'gradient-x 15s ease infinite',
        'gradient-shift': 'gradient-shift 4s linear infinite',
        // Float animations
        'float': 'float 3s ease-in-out infinite',
        'float-hero': 'float-hero 6s ease-in-out infinite',
        'float-slow': 'float-slow 8s ease-in-out infinite',
        // Spin variations
        'spin-slow': 'spin 8s linear infinite',
        'spin-very-slow': 'spin 20s linear infinite',
        'spin-reverse': 'spin 1s linear infinite reverse',
        // Fade animations
        'fade-in': 'fade-in 0.3s ease-out forwards',
        'fade-in-up': 'fade-in-up 0.5s ease-out forwards',
        'fade-in-down': 'fade-in-down 0.5s ease-out forwards',
        'fade-in-scale': 'fade-in-scale 0.3s ease-out forwards',
        // Spring animations
        'spring-in': 'spring-in 0.3s cubic-bezier(0.34, 1.56, 0.64, 1) forwards',
        'spring-bounce': 'spring-bounce 0.6s ease-out',
        // Utility animations
        'shine': 'shine 3s ease-in-out infinite',
        'shimmer': 'shimmer 2s linear infinite',
        'wiggle': 'wiggle 0.5s ease-in-out',
        'shake': 'shake 0.5s ease-in-out',
        'ping-slow': 'ping 2s cubic-bezier(0, 0, 0.2, 1) infinite',
        'typewriter-cursor': 'typewriter-cursor 1s step-end infinite',
      },
      keyframes: {
        // Blob animations
        'blob-move': {
          '0%, 100%': {
            borderRadius: '60% 40% 30% 70% / 60% 30% 70% 40%',
            transform: 'translate(-10px, 10px) scale(1.05)',
          },
          '25%': {
            borderRadius: '40% 60% 70% 30% / 50% 60% 30% 60%',
            transform: 'translate(10px, 10px) scale(1.1)',
          },
          '50%': {
            borderRadius: '30% 60% 70% 40% / 50% 60% 30% 60%',
            transform: 'translate(10px, -10px) scale(1.05)',
          },
          '75%': {
            borderRadius: '40% 30% 50% 60% / 30% 40% 60% 50%',
            transform: 'translate(-10px, -10px) scale(1.1)',
          },
        },
        'blob-morph': {
          '0%, 100%': { borderRadius: '60% 40% 30% 70% / 60% 30% 70% 40%' },
          '25%': { borderRadius: '30% 60% 70% 40% / 50% 60% 30% 60%' },
          '50%': { borderRadius: '50% 60% 30% 60% / 30% 60% 70% 40%' },
          '75%': { borderRadius: '60% 40% 60% 30% / 70% 30% 50% 60%' },
        },
        'blob-float': {
          '0%, 100%': { transform: 'translate(0, 0) scale(1)' },
          '25%': { transform: 'translate(20%, -20%) scale(1.1)' },
          '50%': { transform: 'translate(-10%, 20%) scale(0.9)' },
          '75%': { transform: 'translate(-20%, -10%) scale(1.05)' },
        },
        // Glow animations
        'pulse-glow': {
          '0%, 100%': { opacity: '0.6', transform: 'scale(1)' },
          '50%': { opacity: '1', transform: 'scale(1.05)' },
        },
        'glow-pulse': {
          '0%, 100%': {
            boxShadow: '0 0 20px var(--glow-color, rgba(79, 209, 197, 0.25))',
            opacity: '0.8',
          },
          '50%': {
            boxShadow: '0 0 40px var(--glow-color, rgba(79, 209, 197, 0.5))',
            opacity: '1',
          },
        },
        'text-glow': {
          '0%, 100%': {
            textShadow: '0 0 10px var(--glow-color, hsl(175 70% 50%))',
          },
          '50%': {
            textShadow: '0 0 20px var(--glow-color, hsl(175 70% 50%)), 0 0 40px var(--glow-color, hsl(175 70% 50%))',
          },
        },
        // Gradient animations
        'gradient-flow': {
          '0%, 100%': { backgroundPosition: '0% 50%' },
          '50%': { backgroundPosition: '100% 50%' },
        },
        'gradient-x': {
          '0%, 100%': { backgroundPosition: '0% 50%' },
          '50%': { backgroundPosition: '100% 50%' },
        },
        'gradient-shift': {
          '0%': { backgroundPosition: '0% 0%' },
          '100%': { backgroundPosition: '100% 100%' },
        },
        // Float animations
        'float': {
          '0%, 100%': { transform: 'translateY(0)' },
          '50%': { transform: 'translateY(-10px)' },
        },
        'float-hero': {
          '0%, 100%': { transform: 'translateY(0) rotate(0deg)' },
          '25%': { transform: 'translateY(-5px) rotate(1deg)' },
          '50%': { transform: 'translateY(-10px) rotate(0deg)' },
          '75%': { transform: 'translateY(-5px) rotate(-1deg)' },
        },
        'float-slow': {
          '0%, 100%': { transform: 'translateY(0) translateX(0)' },
          '25%': { transform: 'translateY(-15px) translateX(5px)' },
          '50%': { transform: 'translateY(-20px) translateX(0)' },
          '75%': { transform: 'translateY(-15px) translateX(-5px)' },
        },
        // Fade animations
        'fade-in': {
          from: { opacity: '0' },
          to: { opacity: '1' },
        },
        'fade-in-up': {
          from: { opacity: '0', transform: 'translateY(20px)' },
          to: { opacity: '1', transform: 'translateY(0)' },
        },
        'fade-in-down': {
          from: { opacity: '0', transform: 'translateY(-20px)' },
          to: { opacity: '1', transform: 'translateY(0)' },
        },
        'fade-in-scale': {
          from: { opacity: '0', transform: 'scale(0.95)' },
          to: { opacity: '1', transform: 'scale(1)' },
        },
        // Spring animations
        'spring-in': {
          '0%': { opacity: '0', transform: 'scale(0.95) translateY(-5px)' },
          '60%': { transform: 'scale(1.02) translateY(2px)' },
          '100%': { opacity: '1', transform: 'scale(1) translateY(0)' },
        },
        'spring-bounce': {
          '0%, 100%': { transform: 'translateY(0)' },
          '40%': { transform: 'translateY(-10px)' },
          '60%': { transform: 'translateY(-5px)' },
          '80%': { transform: 'translateY(-2px)' },
        },
        // Utility animations
        'shine': {
          '0%': { transform: 'translateX(-100%) skewX(-15deg)' },
          '100%': { transform: 'translateX(200%) skewX(-15deg)' },
        },
        'shimmer': {
          '0%': { backgroundPosition: '-200% 0' },
          '100%': { backgroundPosition: '200% 0' },
        },
        'wiggle': {
          '0%, 100%': { transform: 'rotate(0deg)' },
          '25%': { transform: 'rotate(-3deg)' },
          '75%': { transform: 'rotate(3deg)' },
        },
        'shake': {
          '0%, 100%': { transform: 'translateX(0)' },
          '10%, 30%, 50%, 70%, 90%': { transform: 'translateX(-5px)' },
          '20%, 40%, 60%, 80%': { transform: 'translateX(5px)' },
        },
        'typewriter-cursor': {
          '0%, 50%': { opacity: '1' },
          '51%, 100%': { opacity: '0' },
        },
      },
      transitionDuration: {
        '400': '400ms',
        '600': '600ms',
        '800': '800ms',
        '900': '900ms',
      },
      transitionTimingFunction: {
        'bounce-in': 'cubic-bezier(0.68, -0.55, 0.265, 1.55)',
        'bounce-out': 'cubic-bezier(0.34, 1.56, 0.64, 1)',
        'smooth': 'cubic-bezier(0.4, 0, 0.2, 1)',
      },
      backgroundImage: {
        'gradient-radial': 'radial-gradient(var(--tw-gradient-stops))',
        'gradient-conic': 'conic-gradient(from 180deg at 50% 50%, var(--tw-gradient-stops))',
        'gradient-mesh': `
          radial-gradient(at 40% 20%, hsl(175 70% 40% / 0.15) 0px, transparent 50%),
          radial-gradient(at 80% 0%, hsl(265 70% 55% / 0.12) 0px, transparent 50%),
          radial-gradient(at 0% 50%, hsl(15 85% 50% / 0.1) 0px, transparent 50%),
          radial-gradient(at 80% 50%, hsl(165 70% 45% / 0.1) 0px, transparent 50%),
          radial-gradient(at 0% 100%, hsl(270 70% 60% / 0.15) 0px, transparent 50%)
        `,
        'gradient-brand': 'linear-gradient(135deg, hsl(15 85% 50%), hsl(165 70% 45%), hsl(175 70% 40%), hsl(265 70% 55%))',
        'gradient-calm': 'linear-gradient(135deg, hsl(220 30% 15%), hsl(225 35% 10%))',
      },
      zIndex: {
        60: '60',
        70: '70',
        80: '80',
        90: '90',
        100: '100',
      },
    },
  },
  plugins: [],
};
