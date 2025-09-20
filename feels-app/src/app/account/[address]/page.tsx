'use client';

import { useParams } from 'next/navigation';
import { useEffect, useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import Link from 'next/link';
import { useConnection } from '@solana/wallet-adapter-react';
import { PublicKey } from '@solana/web3.js';
import { getAssociatedTokenAddress } from '@solana/spl-token';
import { useWallet } from '@solana/wallet-adapter-react';
import { ExternalLink } from 'lucide-react';
import feelsGuyImage from '@/assets/images/feels_guy.png';

interface TokenHolding {
  tokenAddress: string;
  tokenName: string;
  tokenSymbol: string;
  tokenImage: string;
  balance: number;
  decimals: number;
  totalSupply: number;
  percentageOwned: number;
}

interface CreatedToken {
  address: string;
  name: string;
  symbol: string;
  imageUrl: string;
  marketCap: string;
  launched: string;
}

export default function AccountPage() {
  const params = useParams();
  const address = params.address as string;
  const { connection } = useConnection();
  const { publicKey } = useWallet();
  const [tokenHoldings, setTokenHoldings] = useState<TokenHolding[]>([]);
  const [createdTokens, setCreatedTokens] = useState<CreatedToken[]>([]);
  const [loading, setLoading] = useState(true);
  const [validAddress, setValidAddress] = useState(true);
  
  // Check if this is the user's own account
  const isOwnAccount = publicKey && publicKey.toBase58() === address;

  useEffect(() => {
    const fetchAccountData = async () => {
      try {
        // Validate address
        const pubkey = new PublicKey(address);
        
        // TODO: Replace with actual API calls to your indexer
        // For now, using mock data
        const mockHoldings: TokenHolding[] = [
          {
            tokenAddress: 'feelsWojakMvNsD5n2R8rUPzFiHkq9JbgSstPVNkDPGb',
            tokenName: 'Wojak',
            tokenSymbol: 'WOJAK',
            tokenImage: feelsGuyImage.src,
            balance: 1500000,
            decimals: 9,
            totalSupply: 1000000000,
            percentageOwned: 0.15
          },
          {
            tokenAddress: 'feelsPepewJ9nJKy3sLKCqczaTrd2TRnhjxNLPqZB8nu',
            tokenName: 'Pepe',
            tokenSymbol: 'PEPE',
            tokenImage: feelsGuyImage.src,
            balance: 50000000,
            decimals: 9,
            totalSupply: 1000000000,
            percentageOwned: 5.0
          }
        ];

        const mockCreated: CreatedToken[] = [
          {
            address: 'feelsWojakMvNsD5n2R8rUPzFiHkq9JbgSstPVNkDPGb',
            name: 'Wojak',
            symbol: 'WOJAK',
            imageUrl: feelsGuyImage.src,
            marketCap: '$4.2M',
            launched: '3 days ago'
          }
        ];

        setTokenHoldings(mockHoldings);
        setCreatedTokens(mockCreated);
        setLoading(false);
      } catch (error) {
        console.error('Invalid address:', error);
        setValidAddress(false);
        setLoading(false);
      }
    };

    if (address) {
      fetchAccountData();
    }
  }, [address, connection]);

  if (!validAddress) {
    return (
      <div id="invalid-address-container" className="container mx-auto px-4 py-8">
        <div id="invalid-address-content" className="text-center">
          <h1 id="invalid-address-title" className="text-2xl font-bold mb-4">Invalid Address</h1>
          <p id="invalid-address-message" className="text-muted-foreground">The provided address is not a valid Solana address.</p>
        </div>
      </div>
    );
  }

  if (loading) {
    return (
      <div id="loading-container" className="container mx-auto px-4 py-8">
        <div id="loading-skeleton" className="animate-pulse">
          <div id="loading-title-skeleton" className="h-8 bg-muted rounded w-1/4 mb-8"></div>
          <div id="loading-grid-skeleton" className="grid gap-6">
            <div id="loading-card-skeleton-1" className="h-64 bg-muted rounded"></div>
            <div id="loading-card-skeleton-2" className="h-64 bg-muted rounded"></div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div id="account-page-container" className="container mx-auto px-4 py-8">
      <div id="account-header" className="mb-8">
        <h1 id="account-title" className="text-3xl font-bold mb-2">Account Profile</h1>
        <div id="account-address-section" className="flex items-center gap-3 text-sm">
          <a 
            id="account-address-link"
            href={`https://solscan.io/account/${address}?cluster=devnet`}
            target="_blank"
            rel="noopener noreferrer"
            className="text-muted-foreground font-mono break-all hover:text-primary transition-colors flex items-center gap-1"
          >
            {address}
            <ExternalLink id="external-link-icon" className="h-3 w-3 flex-shrink-0" />
          </a>
          {isOwnAccount && (
            <Badge 
              id="own-account-indicator" 
              variant="outline" 
              className="text-xs px-1.5 py-0 h-5 bg-primary/10 text-primary border-primary/20"
            >
              You
            </Badge>
          )}
        </div>
      </div>

      <div id="account-content-grid" className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Token Holdings */}
        <Card id="token-holdings-card" className="h-fit">
          <CardHeader>
            <CardTitle id="token-holdings-title">Token Holdings</CardTitle>
          </CardHeader>
          <CardContent>
            {tokenHoldings.length === 0 ? (
              <p id="no-holdings-message" className="text-muted-foreground">No token holdings found</p>
            ) : (
              <div id="token-holdings-list" className="space-y-4">
                {tokenHoldings.map((holding) => (
                  <Link
                    key={holding.tokenAddress}
                    id={`token-holding-${holding.tokenSymbol.toLowerCase()}`}
                    href={`/token/${holding.tokenAddress}`}
                    className="flex items-center justify-between p-4 rounded-lg border hover:bg-muted/50 transition-colors"
                  >
                    <div className="flex items-center gap-4">
                      <img
                        id={`token-image-${holding.tokenSymbol.toLowerCase()}`}
                        src={holding.tokenImage}
                        alt={holding.tokenName}
                        className="w-12 h-12 rounded-full"
                      />
                      <div>
                        <h3 id={`token-name-${holding.tokenSymbol.toLowerCase()}`} className="font-semibold">{holding.tokenName}</h3>
                        <p id={`token-symbol-${holding.tokenSymbol.toLowerCase()}`} className="text-sm text-muted-foreground">{holding.tokenSymbol}</p>
                      </div>
                    </div>
                    <div className="text-right">
                      <p id={`token-balance-${holding.tokenSymbol.toLowerCase()}`} className="font-semibold">
                        {(holding.balance / Math.pow(10, holding.decimals)).toLocaleString()}
                      </p>
                      <p id={`token-percentage-${holding.tokenSymbol.toLowerCase()}`} className="text-sm text-muted-foreground">
                        {holding.percentageOwned.toFixed(2)}% of supply
                      </p>
                    </div>
                  </Link>
                ))}
              </div>
            )}
          </CardContent>
        </Card>

        {/* Created Tokens */}
        <Card id="created-tokens-card" className="h-fit">
          <CardHeader>
            <CardTitle id="created-tokens-title">Created Tokens</CardTitle>
          </CardHeader>
          <CardContent>
            {createdTokens.length === 0 ? (
              <p id="no-created-tokens-message" className="text-muted-foreground">No tokens created by this account</p>
            ) : (
              <div id="created-tokens-grid" className="grid grid-cols-1 xl:grid-cols-2 gap-4">
                {createdTokens.map((token) => (
                  <Link
                    key={token.address}
                    id={`created-token-${token.symbol.toLowerCase()}`}
                    href={`/token/${token.address}`}
                    className="block group"
                  >
                    <Card id={`created-token-card-${token.symbol.toLowerCase()}`} className="h-full hover:shadow-lg transition-shadow cursor-pointer">
                      <CardHeader className="pb-2">
                        <div className="flex items-center justify-between mb-1">
                          <div id={`created-token-image-container-${token.symbol.toLowerCase()}`} className="w-14 h-14 rounded-full overflow-hidden bg-muted">
                            <img
                              id={`created-token-image-${token.symbol.toLowerCase()}`}
                              src={token.imageUrl}
                              alt={token.name}
                              className="w-full h-full object-cover"
                            />
                          </div>
                          <Badge id={`created-token-launch-badge-${token.symbol.toLowerCase()}`} variant="outline" className="text-xs">
                            {token.launched}
                          </Badge>
                        </div>
                        <div>
                          <h3 id={`created-token-header-${token.symbol.toLowerCase()}`} className="font-semibold flex items-center gap-2">
                            <span id={`created-token-name-${token.symbol.toLowerCase()}`}>{token.name}</span>
                            <span id={`created-token-symbol-${token.symbol.toLowerCase()}`} className="text-sm text-muted-foreground/70">${token.symbol}</span>
                          </h3>
                        </div>
                      </CardHeader>
                      <CardContent className="pt-0">
                        <div id={`created-token-market-cap-container-${token.symbol.toLowerCase()}`} className="flex justify-between items-center">
                          <span id={`created-token-market-cap-label-${token.symbol.toLowerCase()}`} className="text-xs text-muted-foreground">Market Cap</span>
                          <span id={`created-token-market-cap-value-${token.symbol.toLowerCase()}`} className="text-sm font-medium">{token.marketCap}</span>
                        </div>
                      </CardContent>
                    </Card>
                  </Link>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}