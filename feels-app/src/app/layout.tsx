import type { Metadata, Viewport } from 'next';
import localFont from 'next/font/local';
import { JetBrains_Mono } from 'next/font/google';
import './globals.css';
import '../styles/prose-overrides.css';
import 'katex/dist/katex.min.css';
import '../components/content/image-cropper.css';
import { ConditionalNavBar } from '@/components/common/ConditionalNavBar';
import { SolanaWalletProvider } from '@/components/wallet/SolanaWalletProvider';
import { ReactQueryProvider } from '@/components/common/ReactQueryProvider';
import { Toaster } from '@/components/ui/toaster';
import { DataSourceProvider } from '@/contexts/DataSourceContext';
import { SearchProvider } from '@/contexts/SearchContext';
import { DeveloperModeProvider } from '@/contexts/DeveloperModeContext';
import { BackgroundPrefetch } from '@/components/common/BackgroundPrefetch';
import { Footer } from '@/components/common/Footer';
import { ChunkErrorBoundary } from '@/components/common/ChunkErrorBoundary';
import { GlobalHotkeyProvider } from '@/components/common/GlobalHotkeyProvider';
import { LightboxProvider } from '@/components/ui/LightboxProvider';
import { VanityAddressProvider } from '@/contexts/VanityAddressContext';
import dynamic from 'next/dynamic';

// Dynamic import for DevBridge to avoid production bundle impact
const DevBridgeProvider = dynamic(
  () => import('../../tools/devbridge/client/DevBridgeProvider').then(mod => ({ default: mod.DevBridgeProvider }))
);

const DevBridgeWrapper = ({ children }: { children: React.ReactNode }) => {
  const isDevBridgeEnabled = process.env['NEXT_PUBLIC_DEVBRIDGE_ENABLED'] === 'true';
  
  if (isDevBridgeEnabled) {
    return <DevBridgeProvider>{children}</DevBridgeProvider>;
  }
  return <>{children}</>;
};

const terminalGrotesque = localFont({
  src: '../assets/fonts/terminal-grotesque.woff2',
  weight: '100 900',
  style: 'normal',
  variable: '--font-terminal-grotesque',
  display: 'swap',
  fallback: ['ui-sans-serif', 'system-ui', 'sans-serif'],
});

const jetbrainsMono = JetBrains_Mono({
  subsets: ['latin'],
  weight: ['100', '200', '300', '400', '500', '600', '700', '800'],
  style: ['normal', 'italic'],
  variable: '--font-jetbrains-mono',
  display: 'swap',
  fallback: ['ui-monospace', 'SFMono-Regular', 'Menlo', 'Monaco', 'Consolas', 'Liberation Mono', 'Courier New', 'monospace'],
});

export const viewport: Viewport = {
  width: 'device-width',
  initialScale: 1,
  maximumScale: 1,
};

export const metadata: Metadata = {
  title: 'FEELS - Solana DEX',
  description: 'Trade any token on Solana with concentrated liquidity and Jupiter aggregation',
  keywords: ['Solana', 'DeFi', 'AMM', 'Concentrated Liquidity', 'FEELS', 'Jupiter', 'Trading'],
  // Next.js automatically serves favicon.ico from the app directory
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className={`${terminalGrotesque.variable} ${jetbrainsMono.variable}`} data-scroll-behavior="smooth" suppressHydrationWarning={true}>
      <head>
        {/* WASM Preload Links for Fastest Vanity Mining Initialization */}
        <link rel="modulepreload" href="/wasm/vanity_miner_wasm.js" />
        <link rel="prefetch" href="/wasm/vanity_miner_wasm_bg.wasm" />
        <link rel="preload" href="/wasm/vanity-worker.js" as="script" />
        <link rel="preload" href="/wasm/vanity-coordinator.js" as="script" />
        
        <script src="/wallet-patch.js" defer></script>
        <script
          dangerouslySetInnerHTML={{
            __html: `
              // Disable Lit dev mode to prevent console warnings
              // Set it on all possible global objects before Lit loads
              if (typeof globalThis !== 'undefined') {
                globalThis.litDevMode = false;
                globalThis.litDev = false;
              }
              if (typeof window !== 'undefined') {
                window.litDevMode = false;
                window.litDev = false;
              }
              if (typeof global !== 'undefined') {
                global.litDevMode = false;
                global.litDev = false;
              }
              
              // Register service worker for WASM optimization
              if (typeof window !== 'undefined' && 'serviceWorker' in navigator) {
                navigator.serviceWorker.register('/sw.js')
                  .then(registration => {
                    console.log('SW registered:', registration);
                    
                    // Listen for messages from service worker
                    navigator.serviceWorker.addEventListener('message', event => {
                      if (event.data.type === 'wasm-precompiled') {
                        window.__wasmPromise = Promise.resolve(event.data.module);
                        console.log('Pre-compiled WASM received from service worker');
                      }
                    });
                  })
                  .catch(error => console.log('SW registration failed:', error));
              }
              
              // Precompile WASM for instant initialization
              if (typeof window !== 'undefined' && 'WebAssembly' in window) {
                window.__wasmPromise = WebAssembly.compileStreaming(fetch('/wasm/vanity_miner_wasm_bg.wasm'))
                  .catch(() => null); // Fallback gracefully if compilation fails
              }
              
              // Initialize optimized WASM loader and worker pool
              if (typeof window !== 'undefined') {
                import('@/lib/wasmCache').then(({ initializeGlobalWasmLoader }) => {
                  initializeGlobalWasmLoader();
                });
                
                import('@/lib/workerPool').then(({ initializeGlobalWorkerPool }) => {
                  initializeGlobalWorkerPool();
                });
              }
            `,
          }}
        />
      </head>
      <body className={terminalGrotesque.className}>
        <ReactQueryProvider>
          <DeveloperModeProvider>
            <DataSourceProvider>
              <SearchProvider>
                  <SolanaWalletProvider>
                  <VanityAddressProvider>
                    <DevBridgeWrapper>
                      <ChunkErrorBoundary>
                        <GlobalHotkeyProvider>
                          <LightboxProvider>
                            <div className="min-h-screen bg-background flex flex-col">
                        <ConditionalNavBar />
                        <main className="relative z-10 flex-1 flex flex-col">
                          {children}
                        </main>
                        <BackgroundPrefetch />
                        <Footer />
                        </div>
                        <Toaster />
                          </LightboxProvider>
                        </GlobalHotkeyProvider>
                      </ChunkErrorBoundary>
                    </DevBridgeWrapper>
                  </VanityAddressProvider>
                </SolanaWalletProvider>
              </SearchProvider>
            </DataSourceProvider>
          </DeveloperModeProvider>
        </ReactQueryProvider>
      </body>
    </html>
  );
}
