/** @type {import('next').NextConfig} */
const isProd = process.env.NODE_ENV === "production";

const nextConfig = {
  ...(isProd ? { output: "export" } : {}),
  images: {
    unoptimized: true,
  },
  async rewrites() {
    return [
      {
        source: "/api/:path*",
        destination: "http://localhost:3456/api/:path*",
      },
      {
        source: "/ws",
        destination: "http://localhost:3456/ws",
      },
    ];
  },
};

export default nextConfig;
