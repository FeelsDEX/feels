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
import { BackgroundPrefetch } from '@/components/common/BackgroundPrefetch';
import { ChunkErrorBoundary } from '@/components/common/ChunkErrorBoundary';
import { GlobalHotkeyProvider } from '@/components/common/GlobalHotkeyProvider';
import { LightboxProvider } from '@/components/ui/LightboxProvider';
import { GlobalNoMarketsBanner } from '@/components/common/GlobalNoMarketsBanner';
import { VanityAddressProvider } from '@/contexts/VanityAddressContext';
import Link from 'next/link';
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
  icons: {
    icon: [
      { url: './favicon-16x16.png', sizes: '16x16', type: 'image/png' },
      { url: './favicon-32x32.png', sizes: '32x32', type: 'image/png' },
      { url: './favicon.ico', sizes: 'any' },
    ],
    apple: [
      { url: './apple-touch-icon.png', sizes: '180x180', type: 'image/png' },
    ],
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className={`${terminalGrotesque.variable} ${jetbrainsMono.variable}`} data-scroll-behavior="smooth">
      <head>
        <script src="/wallet-patch.js"></script>
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
            `,
          }}
        />
      </head>
      <body className={terminalGrotesque.className}>
        <ReactQueryProvider>
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
                        <GlobalNoMarketsBanner />
                        <main className="relative z-10 flex-1 flex flex-col">
                          {children}
                        </main>
                        <BackgroundPrefetch />
                        <footer className="py-6 md:py-10 mt-auto">
                        <div className="container mx-auto px-4 md:px-6">
                          <div className="relative flex flex-col md:flex-row items-center md:items-center gap-4 md:gap-0">
                            <div className="flex-1 flex justify-center md:justify-start">
                              <div className="flex flex-row md:flex-col space-x-4 md:space-x-0 md:space-y-1 text-center md:text-left">
                                <Link href="/docs" className="text-muted-foreground hover:text-primary transition-colors" prefetch={true}>
                                  docs
                                </Link>
                                <Link href="/blog" className="text-muted-foreground hover:text-primary transition-colors" prefetch={true}>
                                  blog
                                </Link>
                              </div>
                            </div>
                            <p className="text-center text-muted-foreground order-first md:order-none">
                              feels good man
                            </p>
                            <div className="flex-1 flex justify-center md:justify-end">
                              <div className="flex flex-row md:flex-col space-x-4 md:space-x-0 md:space-y-1 text-center md:text-right">
                                <Link href="/info" className="text-muted-foreground hover:text-primary transition-colors" prefetch={true}>
                                  info
                                </Link>
                                <Link href="/control" className="text-muted-foreground hover:text-primary transition-colors" prefetch={true}>
                                  control
                                </Link>
                              </div>
                            </div>
                          </div>
                        </div>
                          </footer>
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
        </ReactQueryProvider>
      </body>
    </html>
  );
}
