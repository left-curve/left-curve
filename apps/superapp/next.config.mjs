/** @type {import('next').NextConfig} */
const nextConfig = {
  images: {
    loader: "custom",
    loaderFile: "./loaders/imageLoader.js"
  }
};

export default nextConfig;
