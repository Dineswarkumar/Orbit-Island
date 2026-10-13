import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        orbit: {
          dark: "#0b0f19",
          card: "rgba(18, 24, 38, 0.75)",
          accent: "#00d2ff",
          border: "rgba(255, 255, 255, 0.12)",
        },
      },
      transitionDuration: {
        "250": "250ms",
      },
    },
  },
  plugins: [],
} satisfies Config;
