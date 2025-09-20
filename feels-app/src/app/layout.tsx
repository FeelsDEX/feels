import type { Metadata } from 'next';
import localFont from 'next/font/local';
import './globals.css';
import { NavBar } from '@/components/NavBar';
import { SolanaWalletProvider } from '@/components/SolanaWalletProvider';
import { ReactQueryProvider } from '@/components/ReactQueryProvider';
import { Toaster } from '@/components/ui/toaster';
import { DataSourceProvider } from '@/contexts/DataSourceContext';
import { SearchProvider } from '@/contexts/SearchContext';
import dynamic from 'next/dynamic';

// Conditionally load DevBridge only in development
const DevBridgeProvider = dynamic(
  () => process.env.NEXT_PUBLIC_DEVBRIDGE_ENABLED === 'true' 
    ? import('../devbridge/client/DevBridgeProvider').then(mod => mod.DevBridgeProvider)
    : Promise.resolve(() => null),
  { ssr: false }
);

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
                <DevBridgeProvider>
                  <div className="min-h-screen bg-background flex flex-col">
                    <div className="relative z-[1000]">
                      <NavBar />
                    </div>
                    <main className="relative z-10 flex-1">
                      {children}
                    </main>
                    <footer className="py-12">
                      <div className="container mx-auto px-4">
                        <div className="relative flex items-center">
                          <div className="flex-1"></div>
                          <p className="text-center text-muted-foreground">
                            feels good man
                          </p>
                          <div className="flex-1 flex justify-end">
                            <div className="flex flex-col space-y-2 text-right">
                              <a href="/info" className="text-muted-foreground hover:text-foreground transition-colors">
                                info
                              </a>
                              <a href="/control" className="text-muted-foreground hover:text-foreground transition-colors">
                                control
                              </a>
                            </div>
                          </div>
                        </div>
                      </div>
                    </footer>
                  </div>
                  <Toaster />
                </DevBridgeProvider>
              </SolanaWalletProvider>
            </SearchProvider>
          </DataSourceProvider>
        </ReactQueryProvider>
      </body>
    </html>
  );
}
