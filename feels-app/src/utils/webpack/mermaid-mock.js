// WEBPACK/NEXT.JS SSR WORKAROUND: Browser-safe mermaid module mock
//
// Purpose: Prevent server-side rendering errors when mermaid is imported.
//
// Issue: The mermaid diagram library manipulates the DOM directly and expects
// a browser environment. During Next.js SSR (Server-Side Rendering), there's no
// DOM available, causing initialization errors.
//
// Solution: This mock provides an empty module during SSR/bundling. The actual
// mermaid library is loaded dynamically in the browser via the MermaidRenderer
// component using dynamic imports with { ssr: false }.
//
// Configuration: Reference this mock in next.config.js webpack config:
//   resolve: { fallback: { 'mermaid': path.resolve(__dirname, 'src/utils/mermaid-mock.js') } }
//
// Related files:
//   - components/MermaidRenderer.tsx (dynamic import with ssr: false)
//   - components/MermaidDiagram.tsx (client-side only component)
//
// This is a standard pattern for Next.js when using browser-only libraries.

// Mock implementation of mermaid for server-side rendering
// This prevents "Cannot find module" errors during SSR

const mermaidMock = {
  initialize: () => {},
  render: () => Promise.resolve({ svg: '<div>Mermaid diagram will render on client</div>' }),
  contentLoaded: () => {},
  run: () => Promise.resolve(),
  parseError: () => {},
  mermaidAPI: {
    initialize: () => {},
    render: () => Promise.resolve({ svg: '<div>Mermaid diagram will render on client</div>' }),
    parseError: () => {},
  }
};

// Support both CommonJS and ES modules
module.exports = mermaidMock;
module.exports.default = mermaidMock;