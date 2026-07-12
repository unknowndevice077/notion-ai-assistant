/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        // Dark neutral palette (confirmed direction: near-black + gray, one accent)
        surface: {
          0: "#111214", // window background
          1: "#17181b", // panel background
          2: "#1c1d20", // card background
          3: "#2a2b2f", // raised / hover
        },
        ink: {
          100: "#e8e8ea", // primary text
          70: "#a8a9ae", // secondary text
          40: "#6b6c72", // muted / placeholder
        },
        accent: {
          DEFAULT: "#4f8cff", // muted blue accent — swap here if a different accent is chosen
          hover: "#6ea0ff",
          muted: "#2a3a5c",
        },
        border: {
          DEFAULT: "#2a2b2f",
          strong: "#38393e",
        },
      },
      fontFamily: {
        sans: [
          "Inter",
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "sans-serif",
        ],
        mono: ["JetBrains Mono", "SFMono-Regular", "monospace"],
      },
      borderRadius: {
        md: "8px",
        lg: "12px",
      },
    },
  },
  plugins: [],
};
