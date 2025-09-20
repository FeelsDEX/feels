'use client';

import { WalletAdapterNetwork } from '@solana/wallet-adapter-base';
import { ConnectionProvider, WalletProvider } from '@solana/wallet-adapter-react';
import { clusterApiUrl } from '@solana/web3.js';
import { useMemo, useState, useEffect } from 'react';

// Lazy load wallet adapter factory
async function createWalletAdapter(walletName: string) {
  switch (walletName) {
    case 'Backpack':
      const { BackpackWalletAdapter } = await import('./BackpackWalletAdapter');
      return new BackpackWalletAdapter();
    case 'MagicEden':
      const { MagicEdenWalletAdapter } = await import('./MagicEdenWalletAdapter');
      return new MagicEdenWalletAdapter();
    case 'Coinbase':
      const { CoinbaseWalletAdapter } = await import('@solana/wallet-adapter-wallets');
      return new CoinbaseWalletAdapter();
    case 'Trust':
      const { TrustWalletAdapter } = await import('@solana/wallet-adapter-wallets');
      return new TrustWalletAdapter();
    case 'OKX':
      const { OKXWalletAdapter } = await import('./OKXWalletAdapter');
      return new OKXWalletAdapter();
    case 'Torus':
      const { TorusWalletAdapter } = await import('@solana/wallet-adapter-wallets');
      return new TorusWalletAdapter();
    case 'Ledger':
      const { LedgerWalletAdapter } = await import('@solana/wallet-adapter-wallets');
      return new LedgerWalletAdapter();
    default:
      throw new Error(`Unknown wallet: ${walletName}`);
  }
}

export function SolanaWalletProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  // You can change this to 'mainnet-beta' or 'testnet' as needed
  const network = WalletAdapterNetwork.Devnet;
  const endpoint = useMemo(() => clusterApiUrl(network), [network]);

  // Lazy load wallet adapters
  const [walletAdapters, setWalletAdapters] = useState<any[]>([]);

  useEffect(() => {
    // Load wallet adapters after initial render
    const loadWallets = async () => {
      const adapterPromises = [
        // Phantom is now a Standard Wallet and doesn't need an adapter
        // import('@solana/wallet-adapter-wallets').then(({ PhantomWalletAdapter }) => new PhantomWalletAdapter()).catch(() => null),
        import('@solana/wallet-adapter-wallets').then(({ SolflareWalletAdapter }) => new SolflareWalletAdapter()).catch(() => null),
        import('./BackpackWalletAdapter').then(({ BackpackWalletAdapter }) => new BackpackWalletAdapter()).catch(() => null),
        import('./MagicEdenWalletAdapter').then(({ MagicEdenWalletAdapter }) => new MagicEdenWalletAdapter()).catch(() => null),
        import('@solana/wallet-adapter-wallets').then(({ CoinbaseWalletAdapter }) => new CoinbaseWalletAdapter()).catch(() => null),
        import('@solana/wallet-adapter-wallets').then(({ TrustWalletAdapter }) => new TrustWalletAdapter()).catch(() => null),
        import('./OKXWalletAdapter').then(({ OKXWalletAdapter }) => new OKXWalletAdapter()).catch(() => null),
        import('@solana/wallet-adapter-wallets').then(({ TorusWalletAdapter }) => new TorusWalletAdapter()).catch(() => null),
        import('@solana/wallet-adapter-wallets').then(({ LedgerWalletAdapter }) => new LedgerWalletAdapter()).catch(() => null),
        import('@solana/wallet-adapter-wallets').then(({ Coin98WalletAdapter }) => new Coin98WalletAdapter()).catch(() => null),
        import('@solana/wallet-adapter-wallets').then(({ BitKeepWalletAdapter }) => new BitKeepWalletAdapter()).catch(() => null),
        import('@solana/wallet-adapter-wallets').then(({ CloverWalletAdapter }) => new CloverWalletAdapter()).catch(() => null),
      ];
      
      const adapters = await Promise.all(adapterPromises);
      // Filter out any failed adapters
      setWalletAdapters(adapters.filter(adapter => adapter !== null));
    };

    // Delay loading to not block initial render
    const timer = setTimeout(loadWallets, 100);
    return () => clearTimeout(timer);
  }, []);

  const wallets = useMemo(() => walletAdapters, [walletAdapters]);

  return (
    <ConnectionProvider endpoint={endpoint}>
      <WalletProvider wallets={wallets} autoConnect>
        {children}
      </WalletProvider>
    </ConnectionProvider>
  );
}

// Export the lazy loader for wallet components to use
export { createWalletAdapter };