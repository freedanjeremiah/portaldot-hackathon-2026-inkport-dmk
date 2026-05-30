import createMDX from '@next/mdx';
import remarkGfm from 'remark-gfm';
import type { NextConfig } from 'next';

const withMDX = createMDX({
  options: {
    remarkPlugins: [remarkGfm],
  },
});

const nextConfig: NextConfig = {
  pageExtensions: ['js', 'jsx', 'ts', 'tsx', 'md', 'mdx'],
  // Optional isolated build dir (defaults to `.next`, so unset = unchanged
  // behaviour). Lets a second concurrent `next dev` use its own dir instead
  // of clobbering a shared `.next`; set NEXT_DIST_DIR to opt in.
  ...(process.env.NEXT_DIST_DIR ? { distDir: process.env.NEXT_DIST_DIR } : {}),
};

export default withMDX(nextConfig);
