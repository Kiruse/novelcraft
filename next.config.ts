import bundleAnalyzer from "@next/bundle-analyzer";
import mdx from "@next/mdx";
import type { NextConfig } from "next";

const withBundleAnalyzer = bundleAnalyzer({
  enabled: process.env.ANALYZE === "true",
});
const withMdx = mdx({});

const nextConfig: NextConfig = {
  pageExtensions: ["ts", "tsx", "mdx"],
};

export default withMdx(
  withBundleAnalyzer(
    nextConfig,
  ),
);
