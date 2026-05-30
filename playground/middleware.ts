import { NextRequest, NextResponse } from 'next/server';

// Permissive CORS for the API routes so a separately-hosted frontend can call
// this backend (https://inkport.philotheephilix.in) cross-origin. Scoped to
// /api/* via the matcher below. OPTIONS preflight is answered here (204);
// other methods pass through with CORS headers attached to the streamed/SSE
// response.
const CORS_HEADERS: Record<string, string> = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
  'Access-Control-Allow-Headers': 'Content-Type, Authorization',
  'Access-Control-Max-Age': '86400',
};

export function middleware(request: NextRequest) {
  if (request.method === 'OPTIONS') {
    return new NextResponse(null, { status: 204, headers: CORS_HEADERS });
  }

  const response = NextResponse.next();
  for (const [k, v] of Object.entries(CORS_HEADERS)) {
    response.headers.set(k, v);
  }
  return response;
}

export const config = {
  matcher: '/api/:path*',
};
