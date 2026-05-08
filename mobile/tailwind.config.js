/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./App.tsx", "./src/**/*.{js,jsx,ts,tsx}"],
  presets: [require("nativewind/preset")],
  theme: {
    extend: {
      colors: {
        // Match ui/ palette — alpha-dark theme
        bg: "#0a0a0a",
        surface: "#161616",
        elevated: "#222222",
        border: "#2a2a2a",
        accent: "#f97316",
        muted: "#71717a",
        text: "#fafafa",
      },
    },
  },
  plugins: [],
};
