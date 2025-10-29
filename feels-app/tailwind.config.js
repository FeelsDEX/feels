/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: ["class"],
  content: [
    './src/pages/**/*.{js,ts,jsx,tsx,mdx}',
    './src/components/**/*.{js,ts,jsx,tsx,mdx}',
    './src/app/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: {
    container: {
      center: true,
      padding: "2rem",
      screens: {
        "2xl": "1400px",
      },
    },
    extend: {
      colors: {
        // shadcn/ui color system
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        secondary: {
          DEFAULT: "hsl(var(--secondary))",
          foreground: "hsl(var(--secondary-foreground))",
        },
        destructive: {
          DEFAULT: "hsl(var(--destructive))",
          foreground: "hsl(var(--destructive-foreground))",
        },
        muted: {
          DEFAULT: "hsl(var(--muted))",
          foreground: "hsl(var(--muted-foreground))",
        },
        accent: {
          DEFAULT: "hsl(var(--accent))",
          foreground: "hsl(var(--accent-foreground))",
        },
        popover: {
          DEFAULT: "hsl(var(--popover))",
          foreground: "hsl(var(--popover-foreground))",
        },
        card: {
          DEFAULT: "hsl(var(--card))",
          foreground: "hsl(var(--card-foreground))",
        },
        // FEELS Brand Colors
        'feels': {
          green: '#5cca39',
          white: '#ffffff', 
          black: '#000000',
          'grey-light': '#808080',
          'grey-dark': '#404040',
        },
        // FEELS Green Color Scale (based on brand green #5cca39 - HSL: 108, 57%, 51%)
        // All shades derived from the brand green for consistency
        'success': {
          50: 'hsl(108, 57%, 95%)',   // Very light green background
          100: 'hsl(108, 57%, 85%)',  // Light green background
          200: 'hsl(108, 57%, 75%)',  // Light green border
          300: 'hsl(108, 57%, 65%)',  // Medium-light green
          400: 'hsl(108, 57%, 58%)',  // Medium green
          500: 'hsl(108, 57%, 51%)',  // Brand green #5cca39 (primary)
          600: 'hsl(108, 57%, 43%)',  // Darker green text
          700: 'hsl(108, 57%, 35%)',  // Dark green text
          800: 'hsl(108, 57%, 27%)',  // Very dark green
          900: 'hsl(108, 57%, 20%)',  // Darkest green
        },
        // FEELS Red Color Scale (based on chart red #ef4444 - HSL: 0, 84%, 60%)
        // Used for negative states, errors, and down movements
        'danger': {
          50: 'hsl(0, 84%, 97%)',     // Very light red background
          100: 'hsl(0, 84%, 90%)',    // Light red background  
          200: 'hsl(0, 84%, 80%)',    // Light red border
          300: 'hsl(0, 84%, 70%)',    // Medium-light red
          400: 'hsl(0, 84%, 65%)',    // Medium red
          500: 'hsl(0, 84%, 60%)',    // Chart red #ef4444 (down candles)
          600: 'hsl(0, 84%, 50%)',    // Darker red text
          700: 'hsl(0, 84%, 40%)',    // Dark red text
          800: 'hsl(0, 84%, 30%)',    // Very dark red
          900: 'hsl(0, 84%, 20%)',    // Darkest red
        },
      },
      borderRadius: {
        'none': '0',
        'pixel': '4px', 
        DEFAULT: '4px',
        'sm': '4px',
        'md': '6px', 
        'lg': '8px',
        'xl': '8px',
        '2xl': '8px',
        '3xl': '8px',
        'full': '9999px',
      },
      fontFamily: {
        sans: ['var(--font-terminal-grotesque)', 'ui-sans-serif', 'system-ui', 'sans-serif'],
        mono: ['var(--font-jetbrains-mono)', 'JetBrains Mono', 'ui-monospace', 'monospace'],
        'terminal': ['var(--font-terminal-grotesque)', 'ui-sans-serif', 'sans-serif'],
      },
      keyframes: {
        "accordion-down": {
          from: { height: "0" },
          to: { height: "var(--radix-accordion-content-height)" },
        },
        "accordion-up": {
          from: { height: "var(--radix-accordion-content-height)" },
          to: { height: "0" },
        },
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideUp: {
          '0%': { transform: 'translateY(10px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
      },
      animation: {
        "accordion-down": "accordion-down 0.2s ease-out",
        "accordion-up": "accordion-up 0.2s ease-out",
        'fade-in': 'fadeIn 0.5s ease-in-out',
        'slide-up': 'slideUp 0.3s ease-out',
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
};
