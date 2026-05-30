// Resolves an API path against an optional separately-hosted backend.
//
// When the frontend is served from the same origin as the API routes (the
// default), NEXT_PUBLIC_BACKEND_URL is empty and apiUrl() returns the path
// unchanged (same-origin fetch). When the frontend is deployed elsewhere, set
// NEXT_PUBLIC_BACKEND_URL=https://inkport.philotheephilix.in so every /api/*
// call targets the toolchain-bearing backend.
//
// NEXT_PUBLIC_ vars are inlined at build time, so this is safe in client code.
const BASE = (process.env.NEXT_PUBLIC_BACKEND_URL ?? '').replace(/\/$/, '');

export function apiUrl(path: string): string {
  if (!BASE) return path;
  return `${BASE}${path.startsWith('/') ? '' : '/'}${path}`;
}
