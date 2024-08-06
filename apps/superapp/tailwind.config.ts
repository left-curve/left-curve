import type { Config } from "tailwindcss";

const config: Config = {
	content: [
		"./app/**/*.{js,ts,jsx,tsx,mdx}",
		"node_modules/@leftcurve/applets/**/*.{js,ts,jsx,tsx}",
		"node_modules/@leftcurve/components/**/*.{js,ts,jsx,tsx}",
	],
	theme: {
		extend: {
			fontFamily: {
				inter: ["var(--font-inter)"],
				grotesk: ["var(--font-grotesk)"],
			},
			colors: {
				primary: {
					DEFAULT: "#006FEE",
					foreground: "#e6f1fe",
					50: "#e6f1fe",
					100: "#cce3fd",
					200: "#99c7fb",
					300: "#66aaf9",
					400: "#338ef7",
					500: "#006FEE",
					600: "#005bc4",
					700: "#004493",
					800: "#002e62",
					900: "#001731",
				},
			},
		},
	},
	darkMode: "class",
	plugins: [],
};
export default config;
