'use client';

import { useState, useEffect } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { WalletModal } from '@/components/wallet/WalletModal';
import { SearchBar } from '@/components/search/SearchBar';
import { getDefaultSwapToken } from '@/utils/get-default-swap-token';
import Link from 'next/link';
import { useRouter, usePathname } from 'next/navigation';
import { useSearchContext } from '@/contexts/SearchContext';
import Image from 'next/image';
import wojakImage from '@/assets/images/wojak.png';

export function NavBar() {
  const [mounted, setMounted] = useState(false);
  const [walletModalOpen, setWalletModalOpen] = useState(false);
  const [swapTokenAddress, setSwapTokenAddress] = useState<string>('');
  const { publicKey, connected } = useWallet();
  const router = useRouter();
  const pathname = usePathname();
  const { isTokenSearchModalOpen } = useSearchContext();

  // Handle hydration
  useEffect(() => {
    setMounted(true);
    // Get default swap token on mount
    setSwapTokenAddress(getDefaultSwapToken());
  }, []);

  // Check if a link is active
  const isLinkActive = (href: string) => {
    if (!pathname) return false;
    
    // Special case for swap - check if we're on any token page
    if (href.includes('/token/')) {
      return pathname.startsWith('/token/');
    }
    
    // Special case for profile - check if we're on any account page
    if (href.includes('/account/')) {
      return pathname.startsWith('/account/');
    }
    
    // For other pages, exact match
    return pathname === href;
  };
  

  // Prevent hydration issues
  if (!mounted) {
    return (
      <header className="pt-2">
        <div className="container mx-auto px-4">
          <div className="flex items-center h-16">
            {/* Left side - Logo */}
            <div className="flex items-center flex-1">
              <div className="px-6 py-3">
                <h1 className="text-5xl font-medium" style={{
                  color: 'transparent',
                  WebkitTextStroke: '1.5px hsl(var(--primary))',
                  ...({
                    textStroke: '1.5px hsl(var(--primary))'
                  } as React.CSSProperties)
                }}>
                  feels
                </h1>
              </div>
            </div>
            
            {/* Center - Search placeholder */}
            <div className="flex-1 max-w-xl mx-8">
              <div className="h-10 bg-muted animate-pulse rounded-lg"></div>
            </div>
            
            {/* Right side - Nav and Wallet */}
            <div className="flex items-center justify-end flex-1">
              <nav className="flex items-center space-x-6 mr-6">
                <div className="h-6 w-12 bg-muted animate-pulse rounded"></div>
                <div className="h-6 w-16 bg-muted animate-pulse rounded"></div>
                <div className="h-6 w-14 bg-muted animate-pulse rounded"></div>
              </nav>
              
              <div className="flex items-center space-x-4">
                <div className="h-9 w-32 bg-muted animate-pulse rounded-md"></div>
                <div className="h-9 w-32 bg-muted animate-pulse rounded-md"></div>
              </div>
            </div>
          </div>
        </div>
      </header>
    );
  }

  return (
    <header id="main-nav-header" className="relative z-[1000] pt-2">
      <div id="nav-container" className="container mx-auto px-4">
        <div id="nav-content-wrapper" className="flex items-center h-16">
          {/* Left side - Logo and Nav */}
          <div id="nav-left-section" className="flex items-center flex-1">
            <div id="logo-wrapper" className="px-6 py-3">
              <Link href="/" className="block">
                <h1 id="feels-logo" className="text-5xl font-medium -mt-0.5 cursor-pointer" style={{
                  color: 'transparent',
                  WebkitTextStroke: '1.5px hsl(var(--primary))',
                  ...({
                    textStroke: '1.5px hsl(var(--primary))'
                  } as React.CSSProperties)
                }}>
                  feels
                </h1>
              </Link>
            </div>
            
            {/* Left Nav */}
            <nav id="left-nav-menu" className="flex items-center space-x-6 ml-4 mt-0.5">
              <Link 
                id="nav-discover-link"
                href="/" 
                className="text-lg font-medium text-foreground hover:text-primary transition-colors relative"
                prefetch={true}
              >
                discover
                {isLinkActive('/') && (
                  <span className="absolute -bottom-1 left-1/2 transform -translate-x-1/2 w-[75%] h-0.5 bg-primary" />
                )}
              </Link>
              <Link 
                id="nav-swap-link"
                href={swapTokenAddress ? `/token/${swapTokenAddress}` : '#'} 
                className="text-lg font-medium text-foreground hover:text-primary transition-colors relative"
                prefetch={true}
                onClick={(e) => {
                  if (!swapTokenAddress) {
                    e.preventDefault();
                    // If for some reason we don't have a token address yet, get one now
                    const tokenAddress = getDefaultSwapToken();
                    setSwapTokenAddress(tokenAddress);
                    router.push(`/token/${tokenAddress}`);
                  }
                }}
              >
                swap
                {isLinkActive(swapTokenAddress ? `/token/${swapTokenAddress}` : '') && (
                  <span className="absolute -bottom-1 left-1/2 transform -translate-x-1/2 w-[75%] h-0.5 bg-primary" />
                )}
              </Link>
            </nav>
          </div>
          
          {/* Center - Search */}
          {!isTokenSearchModalOpen ? (
            <div id="search-section" className="flex-1 max-w-xl mx-8 relative z-[1001]">
              <SearchBar mode="navigation" />
            </div>
          ) : (
            <div className="flex-1 max-w-xl mx-8" />
          )}
          
          {/* Right side - Nav and Wallet */}
          <div id="nav-right-section" className="flex items-center justify-end flex-1">
            <nav id="right-nav-menu" className="flex items-center space-x-6 mr-8 mt-0.5">
              <Link 
                id="nav-launch-link"
                href="/launch" 
                className="text-lg font-medium text-foreground hover:text-primary transition-colors relative mr-3"
                prefetch={true}
              >
                launch
                {isLinkActive('/launch') && (
                  <span className="absolute -bottom-1 left-1/2 transform -translate-x-1/2 w-[75%] h-0.5 bg-primary" />
                )}
              </Link>
              {connected && publicKey && (
                <Link 
                  id="nav-profile-link"
                  href={`/account/${publicKey.toBase58()}`}
                  className="relative hover:opacity-80 transition-opacity"
                  prefetch={true}
                >
                  <Image
                    src={wojakImage}
                    alt="Profile"
                    width={32}
                    height={32}
                    className="rounded-full opacity-90 mt-1"
                  />
                  {isLinkActive(`/account/${publicKey.toBase58()}`) && (
                    <span className="absolute -bottom-1 left-1/2 transform -translate-x-1/2 w-[75%] h-0.5 bg-primary" />
                  )}
                </Link>
              )}
            </nav>
            
            <div id="wallet-section" className="flex items-center">
              {/* Wallet connection button */}
              <button 
                id="wallet-connect-button"
                onClick={() => setWalletModalOpen(true)}
                className="px-4 py-2 bg-white text-black border border-border rounded-lg hover:bg-gray-50"
              >
                {connected && publicKey 
                  ? `${publicKey.toBase58().slice(0, 4)}...${publicKey.toBase58().slice(-4)}`
                  : 'Connect Wallet'
                }
              </button>
              
              {/* Wallet Modal */}
              <WalletModal 
                open={walletModalOpen} 
                onOpenChange={setWalletModalOpen} 
              />
            </div>
          </div>
        </div>
      </div>
    </header>
  );
}