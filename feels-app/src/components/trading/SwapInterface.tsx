'use client';

import React, { useState, useEffect, useMemo } from 'react';
import { Connection } from '@solana/web3.js';
import { useWallet } from '@solana/wallet-adapter-react';
import { Settings, ChevronDown } from 'lucide-react';
import Image from 'next/image';
import { FEELS_TOKENS } from '@/data/tokens';
import { TokenSearchModal } from '@/components/search/TokenSearchModal';
import { TokenSearchResult } from '@/utils/token-search';

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

type TabType = 'swap' | 'limit';

export function SwapInterface({
  onSwapComplete,
  initialFromToken,
  initialToToken,
}: SwapInterfaceProps) {
  const { publicKey, connected } = useWallet();
  const [activeTab, setActiveTab] = useState<TabType>('swap');
  const [fromAmount, setFromAmount] = useState('');
  const [toAmount, setToAmount] = useState('');
  const [fromToken, setFromToken] = useState<TokenInfo | null>(null);
  const [toToken, setToToken] = useState<TokenInfo | null>(null);
  const [showFromTokenSearch, setShowFromTokenSearch] = useState(false);
  const [showToTokenSearch, setShowToTokenSearch] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [limitPrice, setLimitPrice] = useState('');

  // Initialize tokens
  useEffect(() => {
    const defaultFromToken = FEELS_TOKENS.find(t => t.symbol === (initialFromToken || 'SOL'));
    const defaultToToken = FEELS_TOKENS.find(t => t.symbol === (initialToToken || 'USDC'));
    
    // Map FEELS_TOKENS to TokenInfo format with logoURI
    const mapToTokenInfo = (token: typeof FEELS_TOKENS[0]): TokenInfo => ({
      address: token.address,
      symbol: token.symbol,
      name: token.name,
      decimals: token.decimals,
      logoURI: token.imageUrl, // Map imageUrl to logoURI
    });
    
    setFromToken(mapToTokenInfo(defaultFromToken || FEELS_TOKENS[0]));
    setToToken(mapToTokenInfo(defaultToToken || FEELS_TOKENS[1]));
  }, [initialFromToken, initialToToken]);

  // Initialize limit price when switching to limit tab
  useEffect(() => {
    if (activeTab === 'limit' && !limitPrice) {
      // Set a mock market price
      setLimitPrice('50.00'); // Mock SOL/USDC price
    }
    return undefined;
  }, [activeTab, limitPrice]);


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
      console.log('Swap executed:', { fromAmount, toAmount, fromToken, toToken });
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
      <div id="swap-container" className="bg-background border border-border rounded-2xl w-full max-w-[480px] mx-auto">
      {/* Tab Navigation */}
      <div id="tab-navigation" className="flex items-center justify-between border-b border-border">
        <div className="flex">
          {(['swap', 'limit'] as TabType[]).map((tab) => (
            <button
              key={tab}
              id={`tab-button-${tab}`}
              onClick={() => setActiveTab(tab)}
              className={`px-6 py-4 text-sm font-medium capitalize transition-colors relative ${
                activeTab === tab
                  ? 'text-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              {tab}
              {activeTab === tab && (
                <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-primary" />
              )}
            </button>
          ))}
        </div>
        <button id="settings-button" className="p-4 text-muted-foreground hover:text-foreground transition-colors">
          <Settings className="h-5 w-5" />
        </button>
      </div>

      <div className="p-6">
        {/* Limit Price Section - Only show for Limit tab */}
        {activeTab === 'limit' && (
          <div id="limit-price-section" className="space-y-3 mb-4">
            <div className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">When 1</span>
              <div className="flex items-center gap-2">
                {fromToken?.logoURI ? (
                  <Image
                    src={fromToken.logoURI}
                    alt={fromToken.symbol}
                    width={20}
                    height={20}
                    className="rounded-full"
                  />
                ) : (
                  <div className="w-5 h-5 bg-primary rounded-full" />
                )}
                <span className="font-medium">{fromToken?.symbol}</span>
              </div>
              <span className="text-muted-foreground">is worth</span>
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
                    className="rounded-full"
                  />
                ) : (
                  <div className="w-6 h-6 bg-primary rounded-full" />
                )}
                <span className="font-medium text-lg">{toToken?.symbol}</span>
              </div>
            </div>
            
            <div className="flex items-center gap-2">
              <button
                id="limit-market-button"
                onClick={() => {
                  // Set to market price
                  setLimitPrice('4543.76'); // Mock market price
                }}
                className="px-3 py-1 text-sm font-medium border border-border hover:bg-muted/50 rounded-lg transition-colors"
              >
                Market
              </button>
              <button
                id="limit-plus-1-percent"
                onClick={() => {
                  const currentPrice = parseFloat(limitPrice || '4543.76');
                  setLimitPrice((currentPrice * 1.01).toFixed(2));
                }}
                className="px-3 py-1 text-sm font-medium border border-border hover:bg-muted/50 rounded-lg transition-colors"
              >
                +1%
              </button>
              <button
                id="limit-plus-5-percent"
                onClick={() => {
                  const currentPrice = parseFloat(limitPrice || '4543.76');
                  setLimitPrice((currentPrice * 1.05).toFixed(2));
                }}
                className="px-3 py-1 text-sm font-medium border border-border hover:bg-muted/50 rounded-lg transition-colors"
              >
                +5%
              </button>
              <button
                id="limit-plus-10-percent"
                onClick={() => {
                  const currentPrice = parseFloat(limitPrice || '4543.76');
                  setLimitPrice((currentPrice * 1.10).toFixed(2));
                }}
                className="px-3 py-1 text-sm font-medium border border-border hover:bg-muted/50 rounded-lg transition-colors"
              >
                +10%
              </button>
            </div>
          </div>
        )}

        {/* From Section */}
        <div id="from-token-section" className="mb-0">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-muted-foreground mt-2">Sell</span>
            <div className="flex items-center gap-2">
              <button
                id="percentage-25"
                onClick={() => handlePercentageClick(25)}
                className="px-3 py-1 text-xs font-medium border border-border rounded-full hover:bg-muted transition-colors"
              >
                25%
              </button>
              <button
                id="percentage-50"
                onClick={() => handlePercentageClick(50)}
                className="px-3 py-1 text-xs font-medium border border-border rounded-full hover:bg-muted transition-colors"
              >
                50%
              </button>
              <button
                id="percentage-75"
                onClick={() => handlePercentageClick(75)}
                className="px-3 py-1 text-xs font-medium border border-border rounded-full hover:bg-muted transition-colors"
              >
                75%
              </button>
              <button
                id="percentage-max"
                onClick={() => handlePercentageClick(100)}
                className="px-3 py-1 text-xs font-medium border border-border rounded-full hover:bg-muted transition-colors"
              >
                Max
              </button>
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
                      className="rounded-full"
                    />
                  ) : (
                    <div className="w-6 h-6 bg-primary rounded-full" />
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
                d="M12 5L12 19M12 19L6 13M12 19L18 13" 
                stroke="#6B7280" 
                strokeWidth="1.5" 
                strokeLinecap="square" 
                strokeLinejoin="miter"
              />
            </svg>
          </button>
        </div>

        {/* To Section */}
        <div id="to-token-section" className="-mt-8">
          <div className="text-sm text-muted-foreground mb-2">Buy</div>
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
                      className="rounded-full"
                    />
                  ) : (
                    <div className="w-6 h-6 bg-primary rounded-full" />
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

        {/* Swap/Limit Button */}
        <button
          id="swap-button"
          onClick={handleSwap}
          disabled={!connected || !fromAmount || parseFloat(fromAmount) <= 0 || isLoading || (activeTab === 'limit' && !limitPrice)}
          className={`w-full py-4 rounded-xl font-medium text-lg transition-all mt-4 ${
            connected && fromAmount && parseFloat(fromAmount) > 0 && (activeTab !== 'limit' || limitPrice)
              ? 'bg-primary text-primary-foreground hover:opacity-90'
              : 'bg-muted text-muted-foreground cursor-not-allowed'
          }`}
        >
          {!connected
            ? 'Connect Wallet'
            : !fromAmount || parseFloat(fromAmount) <= 0
            ? 'Enter an amount'
            : activeTab === 'limit' && !limitPrice
            ? 'Set limit price'
            : isLoading
            ? activeTab === 'limit' ? 'Placing order...' : 'Swapping...'
            : activeTab === 'limit' ? 'Place limit order' : 'Swap'}
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