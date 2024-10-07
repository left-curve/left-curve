import UnoCSS from "@unocss/postcss";

/** @type {import('postcss-load-config').Config} */
const config = {
  plugins: [UnoCSS()],
};

export default config;
