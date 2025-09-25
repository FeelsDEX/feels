import type { Metadata } from 'next';
import localFont from 'next/font/local';
import './globals.css';
import { NavBar } from '@/components/common/NavBar';
import { SolanaWalletProvider } from '@/components/wallet/SolanaWalletProvider';
import { ReactQueryProvider } from '@/components/common/ReactQueryProvider';
import { Toaster } from '@/components/ui/toaster';
import { DataSourceProvider } from '@/contexts/DataSourceContext';
import { SearchProvider } from '@/contexts/SearchContext';
import { BackgroundPrefetch } from '@/components/common/BackgroundPrefetch';
import Link from 'next/link';
import dynamic from 'next/dynamic';

// Dynamic import for DevBridge to avoid production bundle impact
const DevBridgeProvider = dynamic(
  () => import('../../tools/devbridge/client/DevBridgeProvider').then(mod => ({ default: mod.DevBridgeProvider })),
  { ssr: false }
);

const DevBridgeWrapper = ({ children }: { children: React.ReactNode }) => {
  const isDevBridgeEnabled = process.env.NEXT_PUBLIC_DEVBRIDGE_ENABLED === 'true';
  
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
  fallback: ['ui-monospace', 'SFMono-Regular', 'Menlo', 'Monaco', 'Consolas', 'Liberation Mono', 'Courier New', 'monospace'],
});

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
    <html lang="en" className={terminalGrotesque.variable}>
      <body className={terminalGrotesque.className}>
        <ReactQueryProvider>
          <DataSourceProvider>
            <SearchProvider>
              <SolanaWalletProvider>
                <DevBridgeWrapper>
                  <div className="min-h-screen bg-background flex flex-col">
                    <NavBar />
                    <main className="relative z-10 flex-1 flex flex-col">
                      {children}
                    </main>
                    <BackgroundPrefetch />
                    <footer className="py-10 mt-auto">
                      <div className="container mx-auto px-4">
                        <div className="relative flex items-center">
                          <div className="flex-1"></div>
                          <p className="text-center text-muted-foreground">
                            feels good man
                          </p>
                          <div className="flex-1 flex justify-end">
                            <div className="flex flex-col space-y-1 text-right">
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
                </DevBridgeWrapper>
              </SolanaWalletProvider>
            </SearchProvider>
          </DataSourceProvider>
        </ReactQueryProvider>
      </body>
    </html>
  );
}
