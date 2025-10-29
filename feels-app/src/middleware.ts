// Next.js Middleware for HTTP/2 Server Push and WASM Optimization
import { NextResponse } from 'next/server';
import type { NextRequest } from 'next/server';

export function middleware(request: NextRequest) {
  const response = NextResponse.next();
  const url = request.nextUrl;

  // HTTP/2 Server Push for WASM files on homepage
  if (url.pathname === '/') {
    // Push critical WASM resources for vanity mining
    const links = [
      '</wasm/vanity_miner_wasm_bg.wasm>; rel=preload; as=fetch; crossorigin=anonymous',
      '</wasm/vanity_miner_wasm.js>; rel=modulepreload; crossorigin=anonymous',
      '</wasm/vanity-worker.js>; rel=preload; as=script; crossorigin=anonymous',
      '</wasm/vanity-coordinator.js>; rel=preload; as=script; crossorigin=anonymous'
    ];
    
    response.headers.set('Link', links.join(', '));
    
    // Add optimization headers for WASM files
    response.headers.set('Cross-Origin-Embedder-Policy', 'credentialless');
    response.headers.set('Cross-Origin-Opener-Policy', 'same-origin');
  }

  // Optimize WASM file serving
  if (url.pathname.startsWith('/wasm/')) {
    // Set aggressive caching for WASM files
    response.headers.set('Cache-Control', 'public, max-age=31536000, immutable');
    
    // Enable compression
    response.headers.set('Content-Encoding', 'gzip');
    
    // CORS headers for WASM files
    response.headers.set('Cross-Origin-Resource-Policy', 'cross-origin');
    response.headers.set('Access-Control-Allow-Origin', '*');
    response.headers.set('Access-Control-Allow-Methods', 'GET, HEAD, OPTIONS');
    
    // WASM-specific headers
    if (url.pathname.endsWith('.wasm')) {
      response.headers.set('Content-Type', 'application/wasm');
    }
  }

  // Security headers for all pages
  response.headers.set('X-Content-Type-Options', 'nosniff');
  response.headers.set('X-Frame-Options', 'DENY');
  response.headers.set('X-XSS-Protection', '1; mode=block');
  response.headers.set('Referrer-Policy', 'strict-origin-when-cross-origin');

  // Performance hints
  response.headers.set('X-DNS-Prefetch-Control', 'on');
  
  return response;
}

export const config = {
  matcher: [
    /*
     * Match all request paths except for the ones starting with:
     * - api (API routes)
     * - _next/static (static files)
     * - _next/image (image optimization files)
     * - favicon.ico (favicon file)
     */
    '/((?!api|_next/static|_next/image|favicon.ico).*)',
  ],
};