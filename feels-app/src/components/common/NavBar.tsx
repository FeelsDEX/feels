'use client';

import { useState, useEffect } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { WalletModal } from '@/components/wallet/WalletModal';
import { SearchBar } from '@/components/search/SearchBar';
import { getDefaultSwapToken } from '@/utils/get-default-swap-token';
import Link from 'next/link';
import { useRouter, usePathname } from 'next/navigation';
import { useSearchContext } from '@/contexts/SearchContext';
import { useDataSource } from '@/contexts/DataSourceContext';
import { useDeveloperMode } from '@/contexts/DeveloperModeContext';
import { Menu, X, Search } from 'lucide-react';

export function NavBar() {
  const [mounted, setMounted] = useState(false);
  const [walletModalOpen, setWalletModalOpen] = useState(false);
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [swapTokenAddress, setSwapTokenAddress] = useState<string>('');
  const { publicKey, connected } = useWallet();
  const [displayConnected, setDisplayConnected] = useState(connected);
  const [delayTimeout, setDelayTimeout] = useState<NodeJS.Timeout | null>(null);
  const router = useRouter();
  const pathname = usePathname();
  const { isTokenSearchModalOpen } = useSearchContext();
  const { dataSource, isUsingFallback } = useDataSource();
  const { isDeveloperMode } = useDeveloperMode();

  // Handle hydration
  useEffect(() => {
    setMounted(true);
    // Get default swap token on mount
    setSwapTokenAddress(getDefaultSwapToken(dataSource));
  }, [dataSource]);

  // Handle wallet connection state changes with delay
  useEffect(() => {
    // Clear any existing timeout
    if (delayTimeout) {
      clearTimeout(delayTimeout);
    }

    if (connected && !walletModalOpen) {
      // If wallet is connected and modal is closed, delay showing connected state
      const timeout = setTimeout(() => {
        setDisplayConnected(true);
      }, 300); // 300ms delay
      setDelayTimeout(timeout);
    } else if (!connected) {
      // If wallet is disconnected, update immediately
      setDisplayConnected(false);
      // Clear timeout when disconnected
      if (delayTimeout) {
        clearTimeout(delayTimeout);
        setDelayTimeout(null);
      }
    }

    // Cleanup
    return () => {
      if (delayTimeout) {
        clearTimeout(delayTimeout);
      }
    };
  }, [connected, walletModalOpen]); // Removed delayTimeout from dependencies to prevent infinite loop

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
              <div className="h-10 bg-white animate-pulse rounded-lg"></div>
            </div>
            
            {/* Right side - Nav and Wallet */}
            <div className="flex items-center justify-end flex-1">
              <nav className="flex items-center space-x-6 mr-8 mt-0.5">
                <div className="h-7 w-[52px] bg-white animate-pulse rounded"></div>
              </nav>
              
              <div className="flex items-center">
                <div className="px-4 py-2 bg-white animate-pulse rounded-lg">
                  <div className="h-5 w-[88px]"></div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </header>
    );
  }

  return (
    <header id="main-nav-header" className={`relative pt-2 ${isTokenSearchModalOpen ? 'pointer-events-none' : ''}`}>
      <div id="nav-container" className="container mx-auto px-4 md:px-6">
        <div id="nav-content-wrapper" className="flex items-center h-16">
          {/* Mobile Menu Button */}
          <button
            className="md:hidden p-2 mr-2"
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
            aria-label="Toggle mobile menu"
          >
            {mobileMenuOpen ? <X size={24} /> : <Menu size={24} />}
          </button>

          {/* Mobile Logo */}
          <div id="logo-wrapper" className="md:hidden px-2 py-3 flex items-center">
            <Link href="/" className="block">
              <h1 className="text-4xl font-medium cursor-pointer" style={{
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

          {/* Desktop Layout */}
          <div className="hidden md:flex items-center w-full">
            {/* Left section - Logo and left navigation */}
            <div className="flex items-center">
              <div id="desktop-logo-wrapper" className="px-2 md:px-6 py-3">
                <Link href="/" className="block">
                  <h1 className="text-3xl md:text-5xl font-medium -mt-0.5 cursor-pointer" style={{
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
              <nav id="left-nav-menu" className="flex items-center space-x-8 ml-4 mt-0.5">
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
                      const tokenAddress = getDefaultSwapToken(dataSource);
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
            
            {/* Flex spacer to push right content to the end */}
            <div className="flex-1" />
            
            {/* Right section - Right navigation and wallet */}
            <div className="flex items-center">
              <nav id="right-nav-menu" className="flex items-center space-x-8 mr-10 mt-0.5">
                {/* Connection status badge - only visible in developer mode */}
                {isDeveloperMode && (
                  <div className="flex items-center">
                    <span className={`inline-flex items-center px-2 py-1 text-xs font-medium rounded-full ${
                      isUsingFallback 
                        ? 'bg-yellow-100 text-yellow-800 border border-yellow-200' 
                        : 'bg-success-100 text-success-800 border border-success-200'
                    }`}>
                      <span className={`w-1.5 h-1.5 rounded-full mr-1.5 ${
                        isUsingFallback ? 'bg-yellow-500' : 'bg-success-500'
                      }`} />
                      {isUsingFallback ? 'disconnected' : 'connected'}
                    </span>
                  </div>
                )}
                
                <Link 
                  id="nav-faucet-link"
                  href="/faucet" 
                  className="text-lg font-medium text-foreground hover:text-primary transition-colors relative"
                  prefetch={true}
                >
                  faucet
                  {isLinkActive('/faucet') && (
                    <span className="absolute -bottom-1 left-1/2 transform -translate-x-1/2 w-[75%] h-0.5 bg-primary" />
                  )}
                </Link>
                
                <Link 
                  id="nav-launch-link"
                  href="/launch" 
                  className="text-lg font-medium text-foreground hover:text-primary transition-colors relative"
                  prefetch={true}
                >
                  launch
                  {isLinkActive('/launch') && (
                    <span className="absolute -bottom-1 left-1/2 transform -translate-x-1/2 w-[75%] h-0.5 bg-primary" />
                  )}
                </Link>
              </nav>
              
              <div id="wallet-section" className="flex items-center">
                <button 
                  id="wallet-connect-button"
                  onClick={() => setWalletModalOpen(true)}
                  className="px-2 md:px-3.5 py-2 bg-white text-black border border-border rounded-lg hover:bg-gray-50 text-sm md:text-base"
                >
                  {displayConnected && publicKey 
                    ? `${publicKey.toBase58().slice(0, 4)}...${publicKey.toBase58().slice(-4)}`
                    : 'Connect'
                  }
                </button>
                
                <WalletModal 
                  open={walletModalOpen} 
                  onOpenChange={setWalletModalOpen} 
                />
              </div>
            </div>
          </div>

          {/* Mobile Search Button */}
          <button
            className="md:hidden p-2 ml-auto mr-2"
            onClick={() => router.push('/search')}
            aria-label="Open search"
          >
            <Search size={20} />
          </button>
        </div>
        
        {/* Centered Search Overlay - exactly like search page and TokenSearchModal */}
        {pathname !== '/search' && !pathname.startsWith('/docs') && (
          <div className="hidden md:block absolute top-0 left-0 right-0 pointer-events-none pt-2">
            <div className="container mx-auto px-4">
              <div className="flex items-center h-16">
                <div className="flex-1" />
                <div className="flex-1 max-w-xl mx-8 pointer-events-auto relative z-10">
                  <SearchBar mode="navigation" />
                </div>
                <div className="flex-1" />
              </div>
            </div>
          </div>
        )}

        {/* Mobile Menu Overlay */}
        {mobileMenuOpen && (
          <div className="md:hidden absolute top-16 left-0 right-0 bg-background border-b border-border shadow-lg z-50">
            <nav className="flex flex-col p-4 space-y-4">
              {/* Connection status badge for mobile - only visible in developer mode */}
              {isDeveloperMode && (
                <div className="flex items-center pb-2 border-b border-border">
                  <span className={`inline-flex items-center px-2 py-1 text-xs font-medium rounded-full ${
                    isUsingFallback 
                      ? 'bg-yellow-100 text-yellow-800 border border-yellow-200' 
                      : 'bg-success-100 text-success-800 border border-success-200'
                  }`}>
                    <span className={`w-1.5 h-1.5 rounded-full mr-1.5 ${
                      isUsingFallback ? 'bg-yellow-500' : 'bg-success-500'
                    }`} />
                    {isUsingFallback ? 'disconnected' : 'connected'}
                  </span>
                </div>
              )}
              
              <Link 
                href="/" 
                className="text-lg font-medium text-foreground hover:text-primary transition-colors py-2"
                prefetch={true}
                onClick={() => setMobileMenuOpen(false)}
              >
                discover
                {isLinkActive('/') && (
                  <span className="ml-2 w-2 h-2 bg-primary rounded-full inline-block" />
                )}
              </Link>
              <Link 
                href={swapTokenAddress ? `/token/${swapTokenAddress}` : '#'} 
                className="text-lg font-medium text-foreground hover:text-primary transition-colors py-2"
                prefetch={true}
                onClick={(e) => {
                  if (!swapTokenAddress) {
                    e.preventDefault();
                    const tokenAddress = getDefaultSwapToken(dataSource);
                    setSwapTokenAddress(tokenAddress);
                    router.push(`/token/${tokenAddress}`);
                  }
                  setMobileMenuOpen(false);
                }}
              >
                swap
                {isLinkActive(swapTokenAddress ? `/token/${swapTokenAddress}` : '') && (
                  <span className="ml-2 w-2 h-2 bg-primary rounded-full inline-block" />
                )}
              </Link>
              <Link 
                href="/faucet" 
                className="text-lg font-medium text-foreground hover:text-primary transition-colors py-2"
                prefetch={true}
                onClick={() => setMobileMenuOpen(false)}
              >
                faucet
                {isLinkActive('/faucet') && (
                  <span className="ml-2 w-2 h-2 bg-primary rounded-full inline-block" />
                )}
              </Link>
              <Link 
                href="/launch" 
                className="text-lg font-medium text-foreground hover:text-primary transition-colors py-2"
                prefetch={true}
                onClick={() => setMobileMenuOpen(false)}
              >
                launch
                {isLinkActive('/launch') && (
                  <span className="ml-2 w-2 h-2 bg-primary rounded-full inline-block" />
                )}
              </Link>
              <Link 
                href="/search" 
                className="text-lg font-medium text-foreground hover:text-primary transition-colors py-2 border-t border-border pt-4"
                prefetch={true}
                onClick={() => setMobileMenuOpen(false)}
              >
                search
              </Link>
            </nav>
          </div>
        )}
      </div>
    </header>
  );
}