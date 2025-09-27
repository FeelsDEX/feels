'use client';

import { useState, useEffect, Suspense } from 'react';
import { Program, AnchorProvider, Idl } from '@coral-xyz/anchor';
import { useWallet } from '@solana/wallet-adapter-react';
import { useParams } from 'next/navigation';
import dynamic from 'next/dynamic';
import { getConnection } from '@/services/connection';

import { createFeelsProgram } from '@/sdk/program-workaround';
import { TokenInfo } from '@/services/jupiter-client';
import { FEELS_TOKENS, getTokenByAddress } from '@/data/tokens';
import { TokenHolders } from '@/components/market/TokenHolders';
import { useDataSource } from '@/contexts/DataSourceContext';

// Dynamic imports to prevent SSR issues
const SwapInterface = dynamic(
  () =>
    import('@/components/trading/SwapInterface').then((mod) => ({ default: mod.SwapInterface })),
  {
    ssr: false,
    loading: () => <div className="animate-pulse h-[500px] bg-muted rounded-lg" />,
  }
);

const PriceChart = dynamic(
  () => import('@/components/trading/PriceChart').then((mod) => ({ default: mod.PriceChart })),
  {
    ssr: false,
    loading: () => <div className="animate-pulse h-[400px] bg-muted rounded-lg" />,
  }
);

const LiquidityVisualization = dynamic(
  () =>
    import('@/components/trading/LiquidityVisualization').then((mod) => ({
      default: mod.LiquidityVisualization,
    })),
  {
    ssr: false,
    loading: () => <div className="animate-pulse h-[400px] bg-muted rounded-lg" />,
  }
);

const RecentTradesList = dynamic(
  () =>
    import('@/components/trading/RecentTradesList').then((mod) => ({
      default: mod.RecentTradesList,
    })),
  {
    ssr: false,
    loading: () => <div className="animate-pulse h-[300px] bg-muted rounded-lg" />,
  }
);

function TokenSwapPageContent() {
  const { publicKey, signTransaction, signAllTransactions } = useWallet();
  const params = useParams();
  // const searchParams = useSearchParams(); // Commented out as unused
  const tokenAddress = params['address'] as string;
  // const fromParam = searchParams.get('from'); // Commented out as unused

  // Use singleton connection
  const connection = getConnection();
  const [program, setProgram] = useState<Program<Idl> | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedOutputToken, setSelectedOutputToken] = useState<TokenInfo | null>(null);

  // Initialize program only (connection is already available)
  useEffect(() => {
    async function initializeProgram() {
      try {
        setLoading(true);
        setError(null);

        if (publicKey && signTransaction && signAllTransactions) {
          try {
            // Create provider and program
            const provider = new AnchorProvider(
              connection,
              { publicKey, signTransaction, signAllTransactions } as any,
              { commitment: 'confirmed' }
            );

            // Create program with proper PublicKey
            const feelProgram = createFeelsProgram(provider);
            setProgram(feelProgram);
          } catch (programError) {
            console.error('Failed to create program:', programError);
            setError(
              `Program initialization failed: ${programError instanceof Error ? programError.message : 'Unknown error'}`
            );
          }
        } else {
          // Clear program if wallet is disconnected
          setProgram(null);
        }

        setLoading(false);
      } catch (err) {
        console.error('Failed to initialize:', err);
        setError(err instanceof Error ? err.message : 'Failed to initialize');
        setLoading(false);
      }
    }

    initializeProgram();
  }, [publicKey, signTransaction, signAllTransactions, connection]);

  // Get data source to check if we're in test mode
  const { dataSource } = useDataSource();

  // Validate that the token is a Feels token
  const tokenData = getTokenByAddress(tokenAddress);
  const token = getTokenByAddress(tokenAddress);
  // In test mode, allow all tokens. In indexer mode, only allow Feels tokens
  const isValidToken = token && (dataSource === 'test' || token.isFeelsToken);

  if (!isValidToken) {
    return (
      <div id="token-not-found-container" className="container mx-auto px-4 py-8">
        <div className="flex justify-center">
          <div
            id="token-not-found-card"
            className="bg-card text-card-foreground p-6 pixel-container max-w-[500px] w-full"
          >
            <div className="p-6 text-center">
              <h2 id="token-not-found-title" className="text-xl font-medium mb-4">
                Token Not Found
              </h2>
              <p id="token-not-found-message" className="text-muted-foreground">
                This token is not available on Feels Protocol.
              </p>
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (loading) {
    return (
      <div id="token-loading-container" className="container mx-auto px-4 py-8">
        <div className="flex justify-center">
          <div
            id="token-loading-card"
            className="bg-card text-card-foreground p-6 pixel-container max-w-[500px] w-full"
          >
            <div className="flex items-center justify-center p-8">
              <div className="flex flex-col items-center space-y-4">
                <div
                  id="token-loading-spinner"
                  className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary"
                ></div>
                <p id="token-loading-message" className="text-muted-foreground">
                  Initializing Feels Protocol...
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div id="token-error-container" className="container mx-auto px-4 py-8">
        <div className="flex justify-center">
          <div
            id="token-error-card"
            className="bg-card text-card-foreground p-6 pixel-container max-w-[500px] w-full"
          >
            <div className="p-6">
              <h2
                id="token-error-title"
                className="text-xl font-medium mb-4 flex items-center gap-2"
              >
                <span className="text-xl">Warning</span>
                Connection Error
              </h2>
              <p id="token-error-message" className="text-muted-foreground mb-4">
                {error}
              </p>
              <button
                id="token-error-retry-button"
                onClick={() => window.location.reload()}
                className="btn btn-outline"
              >
                Retry Connection
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div id="token-detail-page" className="container mx-auto px-4 pt-4 pb-8">
      <div id="token-detail-grid" className="grid grid-cols-1 lg:grid-cols-3 gap-8 items-start">
        {/* Left Column: Price Chart, Recent Trades, and Tick Liquidity */}
        <div id="token-info-column" className="order-2 lg:order-1 lg:col-span-2 space-y-8">
          {/* Price Chart */}
          <div id="price-chart-section">
            <PriceChart
              key={`price-chart-${selectedOutputToken?.address || token.address}`}
              tokenSymbol={selectedOutputToken?.symbol || token.symbol}
              tokenAddress={selectedOutputToken?.address || token.address}
              tokenImage={selectedOutputToken?.logoURI || token.imageUrl}
              tokenCreator={tokenData?.creator}
              isFeelsToken={
                dataSource === 'test'
                  ? true
                  : selectedOutputToken?.isFeelsToken || token.isFeelsToken
              }
            />
          </div>


          {/* Recent Trades */}
          <div id="recent-trades-section">
            <RecentTradesList tokenSymbol={token.symbol} tokenAddress={token.address} />
          </div>

          {/* Liquidity Visualization */}
          {connection && (
            <div id="liquidity-visualization-section">
              <LiquidityVisualization
                connection={connection}
                program={program}
                selectedPool={`${token.symbol}/FeelsSOL`}
              />
            </div>
          )}
        </div>

        {/* Right Column: Swap Interface and Holders */}
        <div id="swap-and-holders-column" className="order-1 lg:order-2 lg:col-span-1 space-y-8">
          {/* Swap Interface */}
          {connection && (
            <div id="swap-interface-section">
              <SwapInterface
                connection={connection}
                program={program}
                initialFromToken="SOL"
                initialToToken={token.symbol}
                onSwapComplete={(signature, outputAmount, outputToken) => {
                  console.log('Swap completed:', signature, outputAmount, outputToken);
                  // Update selected token for chart
                  const selectedToken = FEELS_TOKENS.find((t) => t.symbol === outputToken);
                  if (selectedToken) {
                    setSelectedOutputToken({
                      address: selectedToken.address,
                      symbol: selectedToken.symbol,
                      name: selectedToken.name,
                      decimals: selectedToken.decimals,
                      logoURI: selectedToken.imageUrl,
                      isFeelsToken: true,
                    });
                  }
                }}
              />
            </div>
          )}

          {/* Holders directly below Swap */}
          <div id="token-holders-section">
            <TokenHolders tokenAddress={token.address} tokenCreator={tokenData?.creator} />
          </div>
        </div>
      </div>
    </div>
  );
}

export default function TokenSwapPage() {
  return (
    <Suspense
      fallback={
        <div id="token-page-suspense-container" className="container mx-auto px-4 py-8">
          <div className="flex justify-center">
            <div
              id="token-page-suspense-card"
              className="bg-card text-card-foreground p-6 pixel-container max-w-[500px] w-full"
            >
              <div className="flex items-center justify-center p-8">
                <div className="flex flex-col items-center space-y-4">
                  <div
                    id="token-page-suspense-spinner"
                    className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary"
                  ></div>
                  <p id="token-page-suspense-message" className="text-muted-foreground">
                    Loading token page...
                  </p>
                </div>
              </div>
            </div>
          </div>
        </div>
      }
    >
      <TokenSwapPageContent />
    </Suspense>
  );
}
