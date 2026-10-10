import path from "node:path";
import type { NextConfig } from "next";

const config: NextConfig = {
  output: "standalone",
  poweredByHeader: false,
  reactStrictMode: true,
  // The repo root has its own lockfile; this app is its own project.
  turbopack: { root: path.resolve(__dirname) },
};

export default config;
