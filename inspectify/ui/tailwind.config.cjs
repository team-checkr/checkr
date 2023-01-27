const colors = require("tailwindcss/colors");

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.{astro,html,js,jsx,md,mdx,svelte,ts,tsx,vue}"],
  theme: {
    extend: {
      colors: {
        working: colors.gray[500],
        correct: colors.green[700],
        mismatch: colors.orange[500],
        "time-out": colors.blue[700],
        error: colors.red[500],
      },
    },
  },
  plugins: [require("@tailwindcss/typography")],
};
