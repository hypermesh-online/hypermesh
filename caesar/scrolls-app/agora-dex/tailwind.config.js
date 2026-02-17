// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        'caesar-gold': '#FFD700',
        'caesar-red': '#DC143C',
        'caesar-dark': '#1a1a1a',
        'caesar-gray': '#2a2a2a',
      },
      fontFamily: {
        'caesar': ['Inter', 'sans-serif'],
      },
    },
  },
  plugins: [],
}