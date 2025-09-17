/** @type {import('tailwindcss').Config} */
module.exports = {
    darkMode: 'class',
    content: ['./src/**/*.{html,js,svelte,ts}', './index.html'],
    theme: {
        container: {
            center: true,
            padding: '2rem',
            screens: {
                '2xl': '1400px'
            }
        },
        extend: {
            colors: {
                border: 'hsl(var(--border) / <alpha-value>)',
                input: 'hsl(var(--input) / <alpha-value>)',
                ring: 'hsl(var(--ring) / <alpha-value>)',
                background: 'hsl(var(--background) / <alpha-value>)',
                foreground: 'hsl(var(--foreground) / <alpha-value>)',
                primary: {
                    DEFAULT: 'hsl(var(--primary) / <alpha-value>)',
                    foreground: 'hsl(var(--primary-foreground) / <alpha-value>)'
                },
                secondary: {
                    DEFAULT: 'hsl(var(--secondary) / <alpha-value>)',
                    foreground: 'hsl(var(--secondary-foreground) / <alpha-value>)'
                },
                destructive: {
                    DEFAULT: 'hsl(var(--destructive) / <alpha-value>)',
                    foreground: 'hsl(var(--destructive-foreground) / <alpha-value>)'
                },
                muted: {
                    DEFAULT: 'hsl(var(--muted) / <alpha-value>)',
                    foreground: 'hsl(var(--muted-foreground) / <alpha-value>)'
                },
                accent: {
                    DEFAULT: 'hsl(var(--accent) / <alpha-value>)',
                    foreground: 'hsl(var(--accent-foreground) / <alpha-value>)'
                },
                popover: {
                    DEFAULT: 'hsl(var(--popover) / <alpha-value>)',
                    foreground: 'hsl(var(--popover-foreground) / <alpha-value>)'
                },
                card: {
                    DEFAULT: 'hsl(var(--card) / <alpha-value>)',
                    foreground: 'hsl(var(--card-foreground) / <alpha-value>)'
                },
                // Web3 Ecosystem specific colors
                trustchain: {
                    50: '#f0f9ff',
                    500: '#3b82f6',
                    600: '#2563eb',
                    700: '#1d4ed8'
                },
                hypermesh: {
                    50: '#f0fdf4',
                    500: '#22c55e',
                    600: '#16a34a',
                    700: '#15803d'
                },
                caesar: {
                    50: '#fefce8',
                    500: '#eab308',
                    600: '#ca8a04',
                    700: '#a16207'
                },
                stoq: {
                    50: '#fdf4ff',
                    500: '#a855f7',
                    600: '#9333ea',
                    700: '#7c3aed'
                },
                quantum: {
                    50: '#fff1f2',
                    500: '#ef4444',
                    600: '#dc2626',
                    700: '#b91c1c'
                }
            },
            borderRadius: {
                lg: 'var(--radius)',
                md: 'calc(var(--radius) - 2px)',
                sm: 'calc(var(--radius) - 4px)'
            },
            fontFamily: {
                sans: ['Inter', 'system-ui', 'sans-serif'],
                mono: ['JetBrains Mono', 'Fira Code', 'monospace']
            },
            animation: {
                'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
                'bounce-gentle': 'bounce 2s infinite',
                'fade-in': 'fadeIn 0.5s ease-in-out',
                'slide-up': 'slideUp 0.3s ease-out'
            },
            keyframes: {
                fadeIn: {
                    '0%': { opacity: '0' },
                    '100%': { opacity: '1' }
                },
                slideUp: {
                    '0%': { transform: 'translateY(10px)', opacity: '0' },
                    '100%': { transform: 'translateY(0)', opacity: '1' }
                }
            }
        }
    },
    plugins: [
        require('tailwindcss-debug-screens'),
        require('tailwindcss/plugin')(({ addBase }) => {
            addBase({
                ':root': {
                    '--background': '0 0% 100%',
                    '--foreground': '222.2 84% 4.9%',
                    '--card': '0 0% 100%',
                    '--card-foreground': '222.2 84% 4.9%',
                    '--popover': '0 0% 100%',
                    '--popover-foreground': '222.2 84% 4.9%',
                    '--primary': '221.2 83.2% 53.3%',
                    '--primary-foreground': '210 40% 98%',
                    '--secondary': '210 40% 96%',
                    '--secondary-foreground': '222.2 84% 4.9%',
                    '--muted': '210 40% 96%',
                    '--muted-foreground': '215.4 16.3% 46.9%',
                    '--accent': '210 40% 96%',
                    '--accent-foreground': '222.2 84% 4.9%',
                    '--destructive': '0 84.2% 60.2%',
                    '--destructive-foreground': '210 40% 98%',
                    '--border': '214.3 31.8% 91.4%',
                    '--input': '214.3 31.8% 91.4%',
                    '--ring': '221.2 83.2% 53.3%',
                    '--radius': '0.5rem'
                },
                '.dark': {
                    '--background': '222.2 84% 4.9%',
                    '--foreground': '210 40% 98%',
                    '--card': '222.2 84% 4.9%',
                    '--card-foreground': '210 40% 98%',
                    '--popover': '222.2 84% 4.9%',
                    '--popover-foreground': '210 40% 98%',
                    '--primary': '217.2 91.2% 59.8%',
                    '--primary-foreground': '222.2 84% 4.9%',
                    '--secondary': '217.2 32.6% 17.5%',
                    '--secondary-foreground': '210 40% 98%',
                    '--muted': '217.2 32.6% 17.5%',
                    '--muted-foreground': '215 20.2% 65.1%',
                    '--accent': '217.2 32.6% 17.5%',
                    '--accent-foreground': '210 40% 98%',
                    '--destructive': '0 62.8% 30.6%',
                    '--destructive-foreground': '210 40% 98%',
                    '--border': '217.2 32.6% 17.5%',
                    '--input': '217.2 32.6% 17.5%',
                    '--ring': '224.3 76.3% 94.1%'
                }
            })
        })
    ]
};
