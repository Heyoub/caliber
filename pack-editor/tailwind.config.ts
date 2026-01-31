import type { Config } from 'tailwindcss';

export default {
  content: [
    './index.html',
    './src/**/*.{vue,js,ts,jsx,tsx}',
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // HSL-based colors from design system
        coral: {
          200: 'hsl(var(--coral-200))',
          300: 'hsl(var(--coral-300))',
          400: 'hsl(var(--coral-400))',
          500: 'hsl(var(--coral-500))',
          600: 'hsl(var(--coral-600))',
          700: 'hsl(var(--coral-700))',
          800: 'hsl(var(--coral-800))',
        },
        teal: {
          300: 'hsl(var(--teal-300))',
          400: 'hsl(var(--teal-400))',
          500: 'hsl(var(--teal-500))',
          600: 'hsl(var(--teal-600))',
          700: 'hsl(var(--teal-700))',
          800: 'hsl(var(--teal-800))',
        },
        mint: {
          300: 'hsl(var(--mint-300))',
          400: 'hsl(var(--mint-400))',
          500: 'hsl(var(--mint-500))',
          600: 'hsl(var(--mint-600))',
          700: 'hsl(var(--mint-700))',
          800: 'hsl(var(--mint-800))',
        },
        lavender: {
          200: 'hsl(var(--lavender-200))',
          300: 'hsl(var(--lavender-300))',
          400: 'hsl(var(--lavender-400))',
          500: 'hsl(var(--lavender-500))',
          600: 'hsl(var(--lavender-600))',
          700: 'hsl(var(--lavender-700))',
          800: 'hsl(var(--lavender-800))',
        },
        navy: {
          700: 'hsl(var(--navy-700))',
          800: 'hsl(var(--navy-800))',
          900: 'hsl(var(--navy-900))',
        },
        // Semantic aliases
        primary: 'hsl(var(--teal-500))',
        success: 'hsl(var(--mint-500))',
        warning: 'hsl(var(--coral-400))',
        error: 'hsl(var(--coral-600))',
        info: 'hsl(var(--lavender-500))',
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', '-apple-system', 'sans-serif'],
        mono: ['JetBrains Mono', 'Fira Code', 'Consolas', 'monospace'],
        grotesk: ['Space Grotesk', 'Inter', 'sans-serif'],
      },
      animation: {
        'gradient-xy': 'gradient-xy 3s ease infinite',
        'glow-pulse': 'glow-pulse 2s ease-in-out infinite',
        'float': 'float 6s ease-in-out infinite',
        'shimmer': 'shimmer 2s linear infinite',
        'spin-slow': 'spin 8s linear infinite',
      },
      keyframes: {
        'gradient-xy': {
          '0%, 100%': { backgroundPosition: '0% 50%' },
          '50%': { backgroundPosition: '100% 50%' },
        },
        'glow-pulse': {
          '0%, 100%': { opacity: '0.4', transform: 'scale(1)' },
          '50%': { opacity: '0.8', transform: 'scale(1.05)' },
        },
        'float': {
          '0%, 100%': { transform: 'translateY(0)' },
          '50%': { transform: 'translateY(-10px)' },
        },
        'shimmer': {
          '0%': { backgroundPosition: '-200% 0' },
          '100%': { backgroundPosition: '200% 0' },
        },
      },
      backdropBlur: {
        xs: '2px',
      },
      boxShadow: {
        'glow-sm': '0 0 10px rgba(79, 209, 197, 0.3)',
        'glow-md': '0 0 20px rgba(79, 209, 197, 0.4)',
        'glow-lg': '0 0 30px rgba(79, 209, 197, 0.5)',
        'inner-glow': 'inset 0 0 20px rgba(79, 209, 197, 0.2)',
      },
      borderRadius: {
        '4xl': '2rem',
      },
    },
  },
  plugins: [],
} satisfies Config;
