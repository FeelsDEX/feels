// WEBPACK/NEXT.JS SSR WORKAROUND: Browser-safe ws module mock
//
// Purpose: Prevent webpack from bundling the Node.js 'ws' module into browser bundles.
//
// Issue: Some dependencies (like devbridge or indexer clients) may import the Node.js
// 'ws' package, which is server-only. When Next.js bundles these for the browser,
// it fails because 'ws' uses Node.js APIs (net, tls, etc) that don't exist in browsers.
//
// Solution: This mock provides a no-op WebSocket class that allows bundling to succeed.
// The actual functionality is only used on the server side where the real 'ws' module
// is available.
//
// Configuration: Reference this mock in next.config.js webpack config:
//   resolve: { fallback: { 'ws': path.resolve(__dirname, 'src/utils/ws-mock.js') } }
//
// TODO: Refactor code to better separate server-only and client code, potentially
// eliminating the need for this mock.

// Mock for ws module when running in browser
export default class WebSocket {
  constructor() {
    // No-op
  }
  
  on() {
    // No-op
  }
  
  send() {
    // No-op
  }
  
  close() {
    // No-op
  }
}

export const Server = class {
  constructor() {
    // No-op
  }
};