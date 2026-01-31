/** @type {import('tailwindcss').Config} */
export default {
  darkMode: 'class',
  content: [
    './src/**/*.{astro,html,js,jsx,md,mdx,svelte,ts,tsx,vue}',
    './node_modules/@astrojs/*/dist/**/*.astro'
  ],
  theme: {
    screens: {
      'sm': '640px',
      'md': '768px',
      '2xl': '1536px',
    },
    extend: {
      fontFamily: {
        sans: ['OpenDyslexic', 'sans-serif'], // <<< Set default back to OpenDyslexic
        grotesk: ['Space Grotesk', 'sans-serif'], // <<< Define Space Grotesk under its own key
        dyslexic: ['OpenDyslexic'], // Keep your dyslexic key
      },
      fontWeight: {
        normal: '400',
        medium: '500',
        semibold: '600',
        bold: '700',
      },
      colors: {
        // Surface colors
        surface: '#1e293b',
        'surface-dark': '#172033',
        // Brand Colors
        coral: {
          50: 'hsl(0, 100%, 97%)',
          100: 'hsl(0, 100%, 95%)',
          200: 'hsl(0, 100%, 90%)',
          300: 'hsl(0, 95%, 82%)',
          400: 'hsl(0, 90%, 75%)',
          500: 'hsl(0, 88%, 65%)',
          600: 'hsl(0, 85%, 55%)',
          700: 'hsl(0, 80%, 45%)',
          800: 'hsl(0, 75%, 35%)',
          900: 'hsl(0, 70%, 25%)',
        },
        teal: {
          50: 'hsl(190, 90%, 94%)',
          100: 'hsl(190, 90%, 90%)',
          200: 'hsl(190, 90%, 80%)',
          300: 'hsl(190, 90%, 70%)',
          400: 'hsl(190, 90%, 60%)',
          500: 'hsl(190, 92%, 45%)',
          600: 'hsl(190, 90%, 40%)',
          700: 'hsl(190, 85%, 35%)',
          800: 'hsl(190, 80%, 30%)',
          900: 'hsl(190, 75%, 25%)',
        },
        mint: {
          50: 'hsl(165, 65%, 95%)',
          100: 'hsl(165, 65%, 90%)',
          200: 'hsl(165, 65%, 80%)',
          300: 'hsl(165, 65%, 70%)',
          400: 'hsl(165, 65%, 65%)',
          500: 'hsl(165, 65%, 55%)',
          600: 'hsl(165, 65%, 45%)',
          700: 'hsl(165, 60%, 40%)',
          800: 'hsl(165, 55%, 30%)',
          900: 'hsl(165, 50%, 25%)',
        },
        lavender: {
          50: 'hsl(280, 65%, 97%)',
          100: 'hsl(280, 65%, 95%)',
          200: 'hsl(280, 60%, 90%)',
          300: 'hsl(280, 55%, 80%)',
          400: 'hsl(280, 50%, 70%)',
          500: 'hsl(280, 45%, 65%)',
          600: 'hsl(280, 45%, 55%)',
          700: 'hsl(280, 45%, 45%)',
          800: 'hsl(280, 40%, 35%)',
          900: 'hsl(280, 40%, 25%)',
        },
        navy: {
          50: 'hsl(215, 50%, 96%)',
          100: 'hsl(215, 50%, 90%)',
          200: 'hsl(215, 50%, 80%)',
          300: 'hsl(215, 50%, 70%)',
          400: 'hsl(215, 50%, 60%)',
          500: 'hsl(215, 50%, 50%)',
          600: 'hsl(215, 50%, 40%)',
          700: 'hsl(215, 50%, 30%)',
          800: 'hsl(215, 50%, 20%)',
          900: 'hsl(215, 50%, 10%)',
        },
        // Brand Specific Colors - Mapped to your brand variables
        'brand-1': '#5d4561', // hsl(291,16.9%,32.5%)
        'brand-2': '#b182be', // hsl(287,31.6%,62.7%)
        'brand-3': '#066d63', // hsl(174,89.6%,22.5%)
        'brand-4': '#06ccc2', // hsl(177,94.3%,41.2%)
        'brand-5': '#064357', // hsl(195,87.1%,18.2%)
        'brand-6': '#067b9b', // hsl(193,92.5%,31.6%)
        'brand-7': '#943735', // hsl(1,47.3%,39.4%)
        'brand-8': '#c54a52', // hsl(356,51.5%,53.1%)
        // Calm colors for gradients
        calm: {
          gray: 'hsl(240, 6%, 96%)',
          lavender: 'hsl(280, 60%, 91%)',
          coral: 'hsl(0, 60%, 91%)',
        },
        // Emi App Theme Colors
        'emi-primary': '#4FD1C5', // Matches dropdown border color (bright teal)
        'emi-accent': '#5ad5c0',  // Existing accent
        'emi-bg-dark': '#1a2e35', // Matches dropdown background (dark desaturated cyan)
      },
      backgroundImage: {
        'gradient-brand': 'linear-gradient(90deg, var(--tw-gradient-from), var(--tw-gradient-to))',
        'gradient-calm': 'linear-gradient(135deg, hsl(240, 6%, 96%), hsl(280, 60%, 91%) 65%, hsl(0, 60%, 91%) 100%)',
        'gradient-focus': 'linear-gradient(135deg, hsl(215, 50%, 96%), hsl(190, 90%, 94%) 100%)',
        'gradient-ownership': 'linear-gradient(135deg, hsl(215, 50%, 96%), hsl(0, 100%, 97%) 100%)',
        'gradient-mint': 'linear-gradient(135deg, hsl(215, 50%, 96%), hsl(165, 65%, 95%) 100%)',
        'radial-brand': 'radial-gradient(circle, hsl(0, 90%, 75%) 0%, hsl(165, 65%, 65%) 50%, hsl(190, 92%, 45%) 75%, hsl(280, 45%, 65%) 100%)',
      },
      textShadow: {
        none: 'none',
        sm: '0 1px 2px rgba(0, 0, 0, 0.25)',
        md: '0 2px 4px rgba(0, 0, 0, 0.25)',
        lg: '0 4px 8px rgba(0, 0, 0, 0.25)',
      },
      boxShadow: {
        'glow-primary': '0 4px 12px rgba(0, 122, 255, 0.3)',
        'glow-primary-hover': '0 8px 24px rgba(0, 122, 255, 0.4)',
        'glow-accent': '0 4px 12px rgba(79, 209, 197, 0.3)',
        'glow-accent-hover': '0 8px 24px rgba(79, 209, 197, 0.4)',
      },
      backgroundImage: {
        'gradient-brand': 'linear-gradient(90deg, var(--tw-gradient-from), var(--tw-gradient-to))',
        'gradient-calm': 'linear-gradient(135deg, theme("colors.calm.gray"), theme("colors.calm.lavender") 65%, theme("colors.calm.coral") 100%)',
        'gradient-focus': 'linear-gradient(135deg, theme("colors.navy.50"), theme("colors.teal.50") 100%)',
        'gradient-ownership': 'linear-gradient(135deg, theme("colors.navy.50"), theme("colors.coral.50") 100%)',
        'gradient-mint': 'linear-gradient(135deg, theme("colors.navy.50"), theme("colors.mint.50") 100%)',
      },
      animation: {
        'gradient-shift': 'gradientShift 15s ease infinite',
        'blob-move': 'blobMove 20s ease-in-out infinite',
        'pulse-glow': 'pulseGlow 3s ease-in-out infinite',
      },
      keyframes: {
        gradientShift: {
          '0%, 100%': { backgroundPosition: '0% 50%' },
          '50%': { backgroundPosition: '100% 50%' },
        },
        blobMove: {
          '0%, 100%': { 
            backgroundPosition: '0% 50%',
            transform: 'scale(1)',
          },
          '50%': { 
            backgroundPosition: '100% 50%',
            transform: 'scale(1.2)',
          },
        },
        pulseGlow: {
          '0%, 100%': { 
            boxShadow: 'var(--tw-shadow)',
            transform: 'scale(1)',
          },
          '50%': { 
            boxShadow: 'var(--tw-shadow-colored)',
            transform: 'scale(1.02)',
          },
        },
      },
    }
  },
  plugins: [],
};