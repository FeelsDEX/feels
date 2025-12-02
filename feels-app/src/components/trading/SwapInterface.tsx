'use client';

import React, { useState, useEffect, useMemo } from 'react';
import { useSearchParams, useRouter, usePathname } from 'next/navigation';
import { Connection } from '@solana/web3.js';
import { useWallet } from '@solana/wallet-adapter-react';
import { ChevronDown, AlertTriangle } from 'lucide-react';
import Image from 'next/image';
import { FEELS_TOKENS } from '@/constants/mock-tokens';
import { TokenSearchModal } from '@/components/search/TokenSearchModal';
import { TokenSearchResult } from '@/utils/token-search';
// import { useTokenSearch } from '@/hooks/useTokenSearch';

interface SwapInterfaceProps {
  connection: Connection;
  program?: any;
  onSwapComplete?: (signature: string, outputAmount: string, outputToken: string) => void;
  initialFromToken?: string | null;
  initialToToken?: string | null;
}

interface TokenInfo {
  address: string;
  symbol: string;
  name: string;
  decimals: number;
  logoURI?: string;
  balance?: number;
}

type TabType = 'swap' | 'limit' | 'borrow' | 'supply';

export function SwapInterface({
  onSwapComplete,
  initialFromToken,
  initialToToken,
}: SwapInterfaceProps) {
  const { publicKey, connected } = useWallet();
  const searchParams = useSearchParams();
  const router = useRouter();
  const pathname = usePathname();
  
  // Parse initial tab from URL query parameter
  const initialTab = searchParams.get('tab') as TabType | null;
  const validTabs: TabType[] = ['swap', 'limit', 'borrow', 'supply'];
  const defaultTab: TabType = validTabs.includes(initialTab as TabType) ? (initialTab as TabType) : 'swap';
  
  const [activeTab, setActiveTab] = useState<TabType>(defaultTab);
  const [fromAmount, setFromAmount] = useState('');
  const [toAmount, setToAmount] = useState('');
  const [fromToken, setFromToken] = useState<TokenInfo | null>(null);
  const [toToken, setToToken] = useState<TokenInfo | null>(null);
  const [showFromTokenSearch, setShowFromTokenSearch] = useState(false);
  const [showToTokenSearch, setShowToTokenSearch] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [limitPrice, setLimitPrice] = useState('');
  const [slippagePercentage, setSlippagePercentage] = useState('1');
  const [hasInteractedWithSlippage, setHasInteractedWithSlippage] = useState(false);
  
  // Borrow/Supply states
  const [borrowAmount, setBorrowAmount] = useState('');
  const [collateralAmount, setCollateralAmount] = useState('');
  const [supplyAmount, setSupplyAmount] = useState('');
  const borrowApr = '8.5'; // Mock APR (static for now)
  const supplyApy = '6.2'; // Mock APY (static for now)
  const ltv = 75; // Loan-to-value ratio (static for now)
  const healthFactor = 1.5; // Mock health factor (static for now)
  
  // Pre-fetch token data for instant dropdown display
  // const { results: preloadedTokens } = useTokenSearch('');

  // Initialize tokens
  useEffect(() => {
    // Map FEELS_TOKENS to TokenInfo format with logoURI
    const mapToTokenInfo = (token: typeof FEELS_TOKENS[0]): TokenInfo => ({
      address: token.address,
      symbol: token.symbol,
      name: token.name,
      decimals: token.decimals,
      logoURI: token.imageUrl, // Map imageUrl to logoURI
    });
    
    // Find tokens with fallbacks
    const defaultFromToken = FEELS_TOKENS.find(t => t.symbol === (initialFromToken || 'SOL')) || FEELS_TOKENS[0];
    const defaultToToken = FEELS_TOKENS.find(t => t.symbol === (initialToToken || 'USDC')) || FEELS_TOKENS[1];
    
    // Only set if we have valid tokens (FEELS_TOKENS should never be empty, but being defensive)
    if (defaultFromToken) {
      setFromToken(mapToTokenInfo(defaultFromToken));
    }
    if (defaultToToken) {
      setToToken(mapToTokenInfo(defaultToToken));
    }
  }, [initialFromToken, initialToToken]);

  // Initialize limit price when switching to limit tab
  useEffect(() => {
    if (activeTab === 'limit' && !limitPrice) {
      // Set a mock market price
      setLimitPrice('50.00'); // Mock SOL/USDC price
    }
    return undefined;
  }, [activeTab, limitPrice]);

  // Update URL when tab changes
  useEffect(() => {
    const params = new URLSearchParams(searchParams.toString());
    
    // Add or remove tab parameter
    if (activeTab !== 'swap') {
      params.set('tab', activeTab);
    } else {
      params.delete('tab');
    }
    
    // Update URL without triggering navigation
    const newUrl = params.toString() ? `${pathname}?${params.toString()}` : pathname;
    router.replace(newUrl, { scroll: false });
  }, [activeTab, pathname, router, searchParams]);

  // Calculate dollar values (mock)
  const fromDollarValue = useMemo(() => {
    if (!fromAmount || !fromToken) return '$0';
    // Mock calculation
    const value = parseFloat(fromAmount) * 50; // Assuming $50 per token
    return `$${value.toFixed(2)}`;
  }, [fromAmount, fromToken]);

  const toDollarValue = useMemo(() => {
    if (!toAmount || !toToken) return '$0';
    // Mock calculation
    const value = parseFloat(toAmount) * 1; // Assuming $1 per USDC
    return `$${value.toFixed(2)}`;
  }, [toAmount, toToken]);

  // Check if slippage is high (8% or greater)
  const isHighSlippage = parseFloat(slippagePercentage) >= 8;

  const handlePercentageClick = (percentage: number) => {
    // Mock balance
    const balance = 10; // Mock 10 tokens
    const amount = (balance * percentage / 100).toFixed(6);
    setFromAmount(amount);
    // Mock conversion
    setToAmount((parseFloat(amount) * 50).toFixed(2));
  };

  const handleSwap = async () => {
    if (!connected || !publicKey) {
      alert('Please connect your wallet');
      return;
    }

    if (!fromAmount || parseFloat(fromAmount) <= 0) {
      alert('Please enter an amount');
      return;
    }

    setIsLoading(true);
    try {
      // Mock swap
      await new Promise(resolve => setTimeout(resolve, 2000));
      console.log('Swap executed:', { fromAmount, toAmount, fromToken, toToken, slippagePercentage: parseFloat(slippagePercentage) });
      onSwapComplete?.('mock-signature', toAmount, toToken?.symbol || '');
    } catch (error) {
      console.error('Swap failed:', error);
      alert('Swap failed. Please try again.');
    } finally {
      setIsLoading(false);
    }
  };

  const switchTokens = () => {
    setFromToken(toToken);
    setToToken(fromToken);
    setFromAmount(toAmount);
    setToAmount(fromAmount);
  };

  return (
    <>
      <div id="swap-container" className="bg-background border border-border rounded-2xl w-full min-w-0 sm:min-w-[400px] max-w-[480px] mx-auto">
      {/* Tab Navigation */}
      <div id="tab-navigation" className="flex items-center justify-between border-b border-border">
        <div className="flex w-full">
          {(['swap', 'limit', 'borrow', 'supply'] as TabType[]).map((tab, index) => (
            <button
              key={tab}
              id={`tab-button-${tab}`}
              onClick={() => setActiveTab(tab)}
              className={`flex-1 py-4 text-sm font-medium capitalize transition-colors relative ${
                activeTab === tab
                  ? 'text-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              } ${index === 0 ? 'pl-3' : ''} ${index === 3 ? 'pr-3' : ''}`}
            >
              {tab}
              {activeTab === tab && (
                <div className="absolute bottom-0 left-1/2 -translate-x-1/2 w-16 h-0.5 bg-primary" />
              )}
            </button>
          ))}
        </div>
      </div>

      <div className="p-6">
        {/* Limit Price Section - Only show for Limit tab */}
        {activeTab === 'limit' && (
          <div id="limit-price-section" className="space-y-3 mb-4">
            <div className="flex items-center justify-between text-sm">
              <div className="flex items-center gap-2">
                <span className="text-muted-foreground">When 1</span>
                <div className="flex items-center gap-2">
                  {fromToken?.logoURI ? (
                    <Image
                      src={fromToken.logoURI}
                      alt={fromToken.symbol}
                      width={20}
                      height={20}
                      className="rounded"
                      style={{ width: 'auto', height: 'auto' }}
                    />
                  ) : (
                    <div className="w-5 h-5 bg-primary rounded" />
                  )}
                  <span className="font-medium">{fromToken?.symbol}</span>
                </div>
                <span className="text-muted-foreground">is worth</span>
              </div>
              
              <div className="flex items-center gap-1">
                <button
                  id="limit-market-button"
                  onClick={() => {
                    // Set to market price
                    setLimitPrice('4543.76'); // Mock market price
                  }}
                  className="px-2 py-0.5 text-xs font-medium border border-border rounded-full hover:bg-muted transition-colors"
                >
                  Market
                </button>
                <button
                  id="limit-plus-1-percent"
                  onClick={() => {
                    const currentPrice = parseFloat(limitPrice || '4543.76');
                    setLimitPrice((currentPrice * 1.01).toFixed(2));
                  }}
                  className="px-2 py-0.5 text-xs font-medium border border-border rounded-full hover:bg-muted transition-colors"
                >
                  +1%
                </button>
                <button
                  id="limit-plus-5-percent"
                  onClick={() => {
                    const currentPrice = parseFloat(limitPrice || '4543.76');
                    setLimitPrice((currentPrice * 1.05).toFixed(2));
                  }}
                  className="px-2 py-0.5 text-xs font-medium border border-border rounded-full hover:bg-muted transition-colors"
                >
                  +5%
                </button>
                <button
                  id="limit-plus-10-percent"
                  onClick={() => {
                    const currentPrice = parseFloat(limitPrice || '4543.76');
                    setLimitPrice((currentPrice * 1.10).toFixed(2));
                  }}
                  className="px-2 py-0.5 text-xs font-medium border border-border rounded-full hover:bg-muted transition-colors"
                >
                  +10%
                </button>
              </div>
            </div>
            
            <div className="flex items-center gap-2">
              <div className="flex-1 relative">
                <input
                  id="limit-price-input"
                  type="text"
                  value={limitPrice}
                  onChange={(e) => {
                    const value = e.target.value;
                    if (/^\d*\.?\d*$/.test(value)) {
                      setLimitPrice(value);
                    }
                  }}
                  placeholder="0"
                  className="w-full text-4xl font-bold bg-transparent outline-none pr-12"
                  autoComplete="off"
                  autoCorrect="off"
                  autoCapitalize="off"
                  spellCheck="false"
                  inputMode="decimal"
                />
                <div className="absolute right-0 top-1/2 -translate-y-1/2 flex flex-col">
                  <button
                    id="limit-price-increase"
                    onClick={() => {
                      const currentPrice = parseFloat(limitPrice || '0');
                      if (currentPrice > 0) {
                        setLimitPrice((currentPrice + 1).toFixed(2));
                      }
                    }}
                    className="p-0.5 hover:text-primary transition-colors"
                  >
                    <ChevronDown className="h-3 w-3 rotate-180" />
                  </button>
                  <button
                    id="limit-price-decrease"
                    onClick={() => {
                      const currentPrice = parseFloat(limitPrice || '0');
                      if (currentPrice > 1) {
                        setLimitPrice((currentPrice - 1).toFixed(2));
                      }
                    }}
                    className="p-0.5 hover:text-primary transition-colors -mt-1"
                  >
                    <ChevronDown className="h-3 w-3" />
                  </button>
                </div>
              </div>
              <div className="flex items-center gap-2">
                {toToken?.logoURI ? (
                  <Image
                    src={toToken.logoURI}
                    alt={toToken.symbol}
                    width={24}
                    height={24}
                    className="rounded-md"
                    style={{ width: 'auto', height: 'auto' }}
                  />
                ) : (
                  <div className="w-6 h-6 bg-primary rounded-md" />
                )}
                <span className="font-medium text-lg">{toToken?.symbol}</span>
              </div>
            </div>
          </div>
        )}

        {/* Borrow Section - Only show for Borrow tab */}
        {activeTab === 'borrow' && (
          <div id="borrow-section">
            {/* Collateral Section */}
            <div id="collateral-section" className="mb-0">
              <div className="text-sm text-muted-foreground mb-1">Collateral</div>
              <div className="rounded-xl p-4" style={{ backgroundColor: '#f8f8f8' }}>
                <div className="flex items-center justify-between">
                  <div className="flex-1">
                    <input
                      id="collateral-amount-input"
                      type="text"
                      value={collateralAmount}
                      onChange={(e) => {
                        const value = e.target.value;
                        if (/^\d*\.?\d*$/.test(value)) {
                          setCollateralAmount(value);
                          // Auto-calculate borrow amount based on LTV
                          if (value) {
                            const borrow = (parseFloat(value) * ltv / 100).toFixed(2);
                            setBorrowAmount(borrow);
                          }
                        }
                      }}
                      placeholder="0"
                      className="text-3xl font-medium bg-transparent outline-none w-full"
                      autoComplete="off"
                      autoCorrect="off"
                      autoCapitalize="off"
                      spellCheck="false"
                      inputMode="decimal"
                    />
                  </div>

                  <div className="relative">
                    <button
                      id="collateral-token-selector"
                      onClick={() => setShowFromTokenSearch(true)}
                      className="flex items-center gap-2 px-3 py-2 rounded-lg hover:bg-muted transition-colors"
                    >
                      {fromToken?.logoURI ? (
                        <Image
                          src={fromToken.logoURI}
                          alt={fromToken.symbol}
                          width={24}
                          height={24}
                          className="rounded-md"
                          style={{ width: 'auto', height: 'auto' }}
                        />
                      ) : (
                        <div className="w-6 h-6 bg-primary rounded-md" />
                      )}
                      <span className="font-medium">{fromToken?.symbol}</span>
                      <ChevronDown className="h-4 w-4" />
                    </button>
                  </div>
                </div>

                <div className="flex justify-between items-center text-sm text-muted-foreground mt-2">
                  <div>Balance: 10.0 {fromToken?.symbol}</div>
                  <div className="text-xs text-primary">Max LTV: {ltv}%</div>
                </div>
              </div>
            </div>

            {/* Spacer to match swap tab visual spacing */}
            <div className="flex justify-center relative z-10 -mt-1">
              <div className="p-2">
                <div className="h-6 w-6"></div>
              </div>
            </div>

            {/* Borrow Amount Section */}
            <div id="borrow-amount-section" className="-mt-7">
              <div className="text-sm text-muted-foreground mb-1">Borrow</div>
              <div className="rounded-xl p-4" style={{ backgroundColor: '#f8f8f8' }}>
                <div className="flex items-center justify-between">
                  <div className="flex-1">
                    <input
                      id="borrow-amount-input"
                      type="text"
                      value={borrowAmount}
                      onChange={(e) => {
                        const value = e.target.value;
                        if (/^\d*\.?\d*$/.test(value)) {
                          setBorrowAmount(value);
                        }
                      }}
                      placeholder="0"
                      className="text-3xl font-medium bg-transparent outline-none w-full"
                      autoComplete="off"
                      autoCorrect="off"
                      autoCapitalize="off"
                      spellCheck="false"
                      inputMode="decimal"
                    />
                  </div>

                  <div className="relative">
                    <button
                      id="borrow-token-selector"
                      onClick={() => setShowToTokenSearch(true)}
                      className="flex items-center gap-2 px-3 py-2 rounded-lg hover:bg-muted transition-colors"
                    >
                      {toToken?.logoURI ? (
                        <Image
                          src={toToken.logoURI}
                          alt={toToken.symbol}
                          width={24}
                          height={24}
                          className="rounded-md"
                          style={{ width: 'auto', height: 'auto' }}
                        />
                      ) : (
                        <div className="w-6 h-6 bg-primary rounded-md" />
                      )}
                      <span className="font-medium">{toToken?.symbol}</span>
                      <ChevronDown className="h-4 w-4" />
                    </button>
                  </div>
                </div>

                <div className="flex justify-between items-center text-sm text-muted-foreground mt-2">
                  <div>APR: <span className="text-orange-500 font-medium">{borrowApr}%</span></div>
                  <div>Health: <span className={`font-medium ${healthFactor > 1.2 ? 'text-success-500' : healthFactor > 1 ? 'text-yellow-500' : 'text-danger-500'}`}>{healthFactor.toFixed(2)}</span></div>
                </div>
              </div>
            </div>

            {/* Warning */}
            <div className="flex items-start gap-2 p-3 mt-4 bg-orange-500/10 border border-orange-500/20 rounded-lg">
              <AlertTriangle className="h-4 w-4 text-orange-500 mt-0.5 flex-shrink-0" />
              <div className="text-xs text-orange-500">
                <p className="font-medium mb-1">Redenomination Risk</p>
                <p className="opacity-90">While Feels has no liquidations, positions may be redenominated during extreme market stress in order to ensure pool solvency. Redenomination occurs in proportion to net pool duration.</p>
              </div>
            </div>
          </div>
        )}

        {/* Supply Section - Only show for Supply tab */}
        {activeTab === 'supply' && (
          <div id="supply-section">
            {/* Supply Amount Section */}
            <div id="supply-amount-section" className="mb-0">
              <div className="flex items-center justify-between mb-1">
                <div className="text-sm text-muted-foreground">Supply</div>
                <div className="flex items-center gap-1">
                  <button
                    onClick={() => {
                      const balance = 10.0;
                      setSupplyAmount((balance * 0.25).toFixed(6));
                    }}
                    className="px-2 py-0.5 text-xs font-medium border border-border rounded-full hover:bg-muted transition-colors"
                  >
                    25%
                  </button>
                  <button
                    onClick={() => {
                      const balance = 10.0;
                      setSupplyAmount((balance * 0.5).toFixed(6));
                    }}
                    className="px-2 py-0.5 text-xs font-medium border border-border rounded-full hover:bg-muted transition-colors"
                  >
                    50%
                  </button>
                  <button
                    onClick={() => {
                      const balance = 10.0;
                      setSupplyAmount((balance * 0.75).toFixed(6));
                    }}
                    className="px-2 py-0.5 text-xs font-medium border border-border rounded-full hover:bg-muted transition-colors"
                  >
                    75%
                  </button>
                  <button
                    onClick={() => {
                      const balance = 10.0;
                      setSupplyAmount(balance.toString());
                    }}
                    className="px-2 py-0.5 text-xs font-medium border border-border rounded-full hover:bg-muted transition-colors"
                  >
                    All
                  </button>
                </div>
              </div>

              <div className="rounded-xl p-4" style={{ backgroundColor: '#f8f8f8' }}>
                <div className="flex items-center justify-between">
                  <div className="flex-1">
                    <input
                      id="supply-amount-input"
                      type="text"
                      value={supplyAmount}
                      onChange={(e) => {
                        const value = e.target.value;
                        if (/^\d*\.?\d*$/.test(value)) {
                          setSupplyAmount(value);
                        }
                      }}
                      placeholder="0"
                      className="text-3xl font-medium bg-transparent outline-none w-full"
                      autoComplete="off"
                      autoCorrect="off"
                      autoCapitalize="off"
                      spellCheck="false"
                      inputMode="decimal"
                    />
                  </div>

                  <div className="relative">
                    <button
                      id="supply-token-selector"
                      onClick={() => setShowFromTokenSearch(true)}
                      className="flex items-center gap-2 px-3 py-2 rounded-lg hover:bg-muted transition-colors"
                    >
                      {fromToken?.logoURI ? (
                        <Image
                          src={fromToken.logoURI}
                          alt={fromToken.symbol}
                          width={24}
                          height={24}
                          className="rounded-md"
                          style={{ width: 'auto', height: 'auto' }}
                        />
                      ) : (
                        <div className="w-6 h-6 bg-primary rounded-md" />
                      )}
                      <span className="font-medium">{fromToken?.symbol}</span>
                      <ChevronDown className="h-4 w-4" />
                    </button>
                  </div>
                </div>

                <div className="flex justify-between items-center text-sm text-muted-foreground mt-2">
                  <div>Balance: 10.0 {fromToken?.symbol}</div>
                  <div>APY: <span className="text-success-500 font-medium">{supplyApy}%</span></div>
                </div>
              </div>
            </div>

            {/* Supply Info Card */}
            <div className="mt-4 rounded-xl p-4 bg-success-500/10 border border-success-500/20">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <div className="text-xs text-muted-foreground mb-1">APY</div>
                  <div className="text-sm font-medium text-success-500">
                    {supplyApy}%
                  </div>
                </div>
                <div>
                  <div className="text-xs text-muted-foreground mb-1">Total Pool Size</div>
                  <div className="text-sm font-medium">$1.2M</div>
                </div>
              </div>
              <div className="mt-3 pt-3 border-t border-success-500/20">
                <div className="flex items-center gap-2">
                  <div className="w-4 h-4 rounded-full bg-success-500 flex items-center justify-center flex-shrink-0">
                    <span className="text-[10px] text-black font-bold">i</span>
                  </div>
                  <div className="text-xs text-success-700">
                    <p className="font-medium">Earn interest on your deposits</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* From Section - Hide for Borrow/Supply tabs */}
        {(activeTab === 'swap' || activeTab === 'limit') && (
        <>
        <div id="from-token-section" className="mb-0">
          <div className="flex items-center justify-between mb-1">
            <div className="flex items-center gap-1">
              <span className="text-sm text-muted-foreground mr-2">Sell</span>
              <button
                id="percentage-25"
                onClick={() => handlePercentageClick(25)}
                className="px-2 py-0.5 text-xs font-medium border border-border rounded-full hover:bg-muted transition-colors"
              >
                25%
              </button>
              <button
                id="percentage-50"
                onClick={() => handlePercentageClick(50)}
                className="px-2 py-0.5 text-xs font-medium border border-border rounded-full hover:bg-muted transition-colors"
              >
                50%
              </button>
              <button
                id="percentage-75"
                onClick={() => handlePercentageClick(75)}
                className="px-2 py-0.5 text-xs font-medium border border-border rounded-full hover:bg-muted transition-colors"
              >
                75%
              </button>
              <button
                id="percentage-max"
                onClick={() => handlePercentageClick(100)}
                className="px-2 py-0.5 text-xs font-medium border border-border rounded-full hover:bg-muted transition-colors"
              >
                All
              </button>
            </div>
            
            {/* Slippage controls */}
            <div className="flex items-center gap-1.5">
              <div className="relative group w-4 h-4">
                {isHighSlippage && (
                  <>
                    <AlertTriangle className="h-4 w-4 text-danger-500" />
                    <div className="absolute left-1/2 -translate-x-1/2 bottom-full mb-2 px-3 py-2 bg-gray-900 text-white text-xs rounded-md whitespace-nowrap opacity-0 group-hover:opacity-100 pointer-events-none transition-opacity z-10">
                      Warning: high max slippage selected
                      <div className="absolute left-1/2 -translate-x-1/2 top-full w-0 h-0 border-l-[5px] border-l-transparent border-r-[5px] border-r-transparent border-t-[5px] border-t-gray-900"></div>
                    </div>
                  </>
                )}
              </div>
              <label htmlFor="slippage-input" className="text-sm text-muted-foreground whitespace-nowrap">Max slippage:</label>
              <div className="relative">
                <input
                  id="slippage-input"
                  type="text"
                  value={slippagePercentage}
                  onChange={(e) => {
                    const value = e.target.value;
                    // Allow decimal numbers with up to 2 decimal places
                    if (/^\d*\.?\d{0,2}$/.test(value)) {
                      setSlippagePercentage(value);
                      if (value !== '') {
                        setHasInteractedWithSlippage(true);
                      }
                    }
                  }}
                  onFocus={() => {
                    // Clear the value on first interaction
                    if (!hasInteractedWithSlippage && slippagePercentage === '1') {
                      setSlippagePercentage('');
                      setHasInteractedWithSlippage(true);
                    }
                  }}
                  onBlur={() => {
                    // Reset to 1 if empty
                    if (!slippagePercentage || parseFloat(slippagePercentage) === 0) {
                      setSlippagePercentage('1');
                      setHasInteractedWithSlippage(false);
                    }
                  }}
                  placeholder="1.0"
                  className={`feels-input flex h-6 w-10 rounded-lg border bg-background pl-1 pr-4 py-0.5 text-sm placeholder:text-muted-foreground/60 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50 text-center transition-colors ${isHighSlippage ? 'slippage-warning' : ''}`}
                  autoComplete="off"
                  autoCorrect="off"
                  autoCapitalize="off"
                  spellCheck="false"
                  inputMode="decimal"
                />
                <span className="absolute right-2 top-1/2 -translate-y-1/2 text-sm text-muted-foreground pointer-events-none">%</span>
              </div>
            </div>
          </div>

          <div className="rounded-xl p-4" style={{ backgroundColor: '#f8f8f8' }}>
            <div className="flex items-center justify-between">
              <div className="flex-1">
                <input
                  id="from-amount-input"
                  type="text"
                  value={fromAmount}
                  onChange={(e) => {
                    const value = e.target.value;
                    if (/^\d*\.?\d*$/.test(value)) {
                      setFromAmount(value);
                      // Mock conversion
                      setToAmount(value ? (parseFloat(value) * 50).toFixed(2) : '');
                    }
                  }}
                  placeholder="0"
                  className="text-3xl font-medium bg-transparent outline-none w-full"
                  autoComplete="off"
                  autoCorrect="off"
                  autoCapitalize="off"
                  spellCheck="false"
                  inputMode="decimal"
                />
              </div>

              <div className="relative">
                <button
                  id="from-token-selector"
                  onClick={() => setShowFromTokenSearch(true)}
                  className="flex items-center gap-2 px-3 py-2 rounded-lg hover:bg-muted transition-colors"
                >
                  {fromToken?.logoURI ? (
                    <Image
                      src={fromToken.logoURI}
                      alt={fromToken.symbol}
                      width={24}
                      height={24}
                      className="rounded-md"
                      style={{ width: 'auto', height: 'auto' }}
                    />
                  ) : (
                    <div className="w-6 h-6 bg-primary rounded-md" />
                  )}
                  <span className="font-medium">{fromToken?.symbol}</span>
                  <ChevronDown className="h-4 w-4" />
                </button>
              </div>
            </div>

            <div className="flex justify-between items-center text-sm text-muted-foreground mt-2">
              <div>{fromDollarValue}</div>
              <div>Balance: 10.0 {fromToken?.symbol}</div>
            </div>
          </div>
        </div>

        {/* Switch Button */}
        <div className="flex justify-center relative z-10 -mt-1">
          <button
            id="switch-tokens-button"
            onClick={switchTokens}
            className="p-2 rounded hover:rotate-180 transition-all duration-300"
            style={{ imageRendering: 'pixelated' }}
          >
            <svg 
              width="24" 
              height="24" 
              viewBox="0 0 24 24" 
              fill="none" 
              xmlns="http://www.w3.org/2000/svg"
              className="h-6 w-6"
              style={{ shapeRendering: 'crispEdges' }}
            >
              <path 
                d="M12 7L12 19M12 19L6 13M12 19L18 13" 
                stroke="#4B5563" 
                strokeWidth="1.5" 
                strokeLinecap="square" 
                strokeLinejoin="miter"
              />
            </svg>
          </button>
        </div>

        {/* To Section - Hide for Borrow/Supply tabs */}
        <div id="to-token-section" className="-mt-7">
          <div className="text-sm text-muted-foreground mb-1">Buy</div>
          <div className="rounded-xl p-4" style={{ backgroundColor: '#f8f8f8' }}>
            <div className="flex items-center justify-between">
              <div className="flex-1">
                <input
                  id="to-amount-input"
                  type="text"
                  value={toAmount}
                  onChange={(e) => {
                    const value = e.target.value;
                    if (/^\d*\.?\d*$/.test(value)) {
                      setToAmount(value);
                      // Mock reverse conversion
                      setFromAmount(value ? (parseFloat(value) / 50).toFixed(6) : '');
                    }
                  }}
                  placeholder="0"
                  className="text-3xl font-medium bg-transparent outline-none w-full"
                  autoComplete="off"
                  autoCorrect="off"
                  autoCapitalize="off"
                  spellCheck="false"
                  inputMode="decimal"
                />
              </div>

              <div className="relative">
                <button
                  id="to-token-selector"
                  onClick={() => setShowToTokenSearch(true)}
                  className="flex items-center gap-2 px-3 py-2 rounded-lg hover:bg-muted transition-colors"
                >
                  {toToken?.logoURI ? (
                    <Image
                      src={toToken.logoURI}
                      alt={toToken.symbol}
                      width={24}
                      height={24}
                      className="rounded-md"
                      style={{ width: 'auto', height: 'auto' }}
                    />
                  ) : (
                    <div className="w-6 h-6 bg-primary rounded-md" />
                  )}
                  <span className="font-medium">{toToken?.symbol}</span>
                  <ChevronDown className="h-4 w-4" />
                </button>
              </div>
            </div>

            <div className="flex justify-between items-center text-sm text-muted-foreground mt-2">
              <div>{toDollarValue}</div>
              <div>Balance: 0 {toToken?.symbol}</div>
            </div>
          </div>
        </div>
        </>
        )}

        {/* Action Button */}
        <button
          id="swap-button"
          onClick={handleSwap}
          disabled={
            !connected || 
            isLoading || 
            (activeTab === 'swap' && (!fromAmount || parseFloat(fromAmount) <= 0)) ||
            (activeTab === 'limit' && (!fromAmount || parseFloat(fromAmount) <= 0 || !limitPrice)) ||
            (activeTab === 'borrow' && (!collateralAmount || parseFloat(collateralAmount) <= 0 || !borrowAmount)) ||
            (activeTab === 'supply' && (!supplyAmount || parseFloat(supplyAmount) <= 0))
          }
          className={`w-full py-4 rounded-xl font-medium text-lg transition-all mt-4 ${
            connected && !isLoading &&
            ((activeTab === 'swap' && fromAmount && parseFloat(fromAmount) > 0) ||
             (activeTab === 'limit' && fromAmount && parseFloat(fromAmount) > 0 && limitPrice) ||
             (activeTab === 'borrow' && collateralAmount && parseFloat(collateralAmount) > 0 && borrowAmount) ||
             (activeTab === 'supply' && supplyAmount && parseFloat(supplyAmount) > 0))
              ? 'bg-primary text-primary-foreground hover:opacity-90'
              : 'bg-muted text-muted-foreground cursor-not-allowed'
          }`}
        >
          {!connected
            ? 'Connect Wallet'
            : isLoading
            ? activeTab === 'limit' ? 'Placing order...' : activeTab === 'borrow' ? 'Borrowing...' : activeTab === 'supply' ? 'Supplying...' : 'Swapping...'
            : activeTab === 'swap'
            ? (!fromAmount || parseFloat(fromAmount) <= 0 ? 'Enter an amount' : 'Swap')
            : activeTab === 'limit'
            ? (!fromAmount || parseFloat(fromAmount) <= 0 ? 'Enter an amount' : !limitPrice ? 'Set limit price' : 'Place limit order')
            : activeTab === 'borrow'
            ? (!collateralAmount || parseFloat(collateralAmount) <= 0 ? 'Enter collateral amount' : !borrowAmount ? 'Enter borrow amount' : 'Borrow')
            : activeTab === 'supply'
            ? (!supplyAmount || parseFloat(supplyAmount) <= 0 ? 'Enter supply amount' : 'Supply')
            : 'Swap'}
        </button>
      </div>
      </div>
      
      {/* Token Search Modals - rendered outside main container for proper z-index stacking */}
      {showFromTokenSearch && (
        <div id="swap-from-token-modal-wrapper" className="fixed inset-0 z-[1200] pointer-events-none">
          <div id="swap-from-token-modal-inner" className="pointer-events-auto">
            <TokenSearchModal
              isOpen={showFromTokenSearch}
              onClose={() => setShowFromTokenSearch(false)}
              onSelect={(token: TokenSearchResult) => {
                setFromToken({
                  address: token.address,
                  symbol: token.symbol,
                  name: token.name,
                  decimals: token.decimals || 9,
                  logoURI: token.imageUrl,
                });
                setShowFromTokenSearch(false);
              }}
              excludeAddress={toToken?.address}
              placeholder="Search for a token to sell..."
            />
          </div>
        </div>
      )}
      
      {showToTokenSearch && (
        <div id="swap-to-token-modal-wrapper" className="fixed inset-0 z-[1200] pointer-events-none">
          <div id="swap-to-token-modal-inner" className="pointer-events-auto">
            <TokenSearchModal
              isOpen={showToTokenSearch}
              onClose={() => setShowToTokenSearch(false)}
              onSelect={(token: TokenSearchResult) => {
                setToToken({
                  address: token.address,
                  symbol: token.symbol,
                  name: token.name,
                  decimals: token.decimals || 9,
                  logoURI: token.imageUrl,
                });
                setShowToTokenSearch(false);
              }}
              excludeAddress={fromToken?.address}
              placeholder="Search for a token to buy..."
            />
          </div>
        </div>
      )}
    </>
  );
}