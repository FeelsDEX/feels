'use client';

import { useParams } from 'next/navigation';
import { useEffect, useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import Link from 'next/link';
import { useConnection } from '@solana/wallet-adapter-react';
import { PublicKey } from '@solana/web3.js';
import { useWallet } from '@solana/wallet-adapter-react';
import { ExternalLink } from 'lucide-react';
import feelsGuyImage from '@/assets/images/feels_guy.png';
import wojakImage from '@/assets/images/wojak_original.jpg';
import chadImage from '@/assets/images/chad.png';
import npcWojakImage from '@/assets/images/npc_wojak.png';

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
  const address = params['address'] as string;
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
        new PublicKey(address); // Throws if invalid
        
        // TODO: Replace with actual API calls to your indexer
        // For now, using mock data - these addresses match mock-tokens.ts
        const mockHoldings: TokenHolding[] = [
          {
            tokenAddress: 'WojakMvNsD5n2R8rUPzFiHkq9JbgSstPVNkDPGb1feel',
            tokenName: 'Wojak',
            tokenSymbol: 'WOJAK',
            tokenImage: wojakImage.src,
            balance: 1500000,
            decimals: 9,
            totalSupply: 100000000,
            percentageOwned: 1.5
          },
          {
            tokenAddress: 'PepewJ9nJKy3sLKCqczaTrd2TRnhjxNLPqZB8nu2feel',
            tokenName: 'Pepe',
            tokenSymbol: 'PEPE',
            tokenImage: feelsGuyImage.src,
            balance: 50000000,
            decimals: 9,
            totalSupply: 100000000,
            percentageOwned: 50.0
          },
          {
            tokenAddress: 'ChadGPT4NL8z3xZpYjQcBJknmggY3htVKe3SUBz1feel',
            tokenName: 'Chad',
            tokenSymbol: 'CHAD',
            tokenImage: chadImage.src,
            balance: 25000000,
            decimals: 9,
            totalSupply: 100000000,
            percentageOwned: 25.0
          },
          {
            tokenAddress: 'NPCfQ2XbTDN4bWoFZCTQDrdgnDVXKyVGaBPc8Qy7feel',
            tokenName: 'NPC',
            tokenSymbol: 'NPC',
            tokenImage: npcWojakImage.src,
            balance: 10000000,
            decimals: 9,
            totalSupply: 100000000,
            percentageOwned: 10.0
          }
        ];

        const mockCreated: CreatedToken[] = [
          {
            address: 'WojakMvNsD5n2R8rUPzFiHkq9JbgSstPVNkDPGb1feel',
            name: 'Wojak',
            symbol: 'WOJAK',
            imageUrl: wojakImage.src,
            marketCap: '$4.2M',
            launched: '3 days ago'
          },
          {
            address: 'PepewJ9nJKy3sLKCqczaTrd2TRnhjxNLPqZB8nu2feel',
            name: 'Pepe',
            symbol: 'PEPE',
            imageUrl: feelsGuyImage.src,
            marketCap: '$890K',
            launched: '5 days ago'
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
                    className="flex items-center justify-between p-4 rounded-lg border hover:shadow-md hover:border-primary/50 transition-all"
                  >
                    <div className="flex items-center gap-4">
                      <img
                        id={`token-image-${holding.tokenSymbol.toLowerCase()}`}
                        src={holding.tokenImage}
                        alt={holding.tokenName}
                        className="w-12 h-12 rounded-lg object-cover"
                        onError={(e) => {
                          const target = e.target as HTMLImageElement;
                          target.src = feelsGuyImage.src;
                        }}
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
              <div id="created-tokens-grid" className="space-y-3">
                {createdTokens.map((token) => (
                  <Link
                    key={token.address}
                    id={`created-token-${token.symbol.toLowerCase()}`}
                    href={`/token/${token.address}`}
                    className="block"
                  >
                    <div className="border rounded-lg p-3 hover:shadow-md hover:border-primary/50 transition-all cursor-pointer">
                      <div className="flex items-center gap-3">
                        {/* Token Image - square with rounded corners */}
                        <div className="relative h-12 w-12 flex-shrink-0">
                          <img
                            id={`created-token-image-${token.symbol.toLowerCase()}`}
                            src={token.imageUrl}
                            alt={token.name}
                            className="w-full h-full rounded-lg object-cover"
                            onError={(e) => {
                              const target = e.target as HTMLImageElement;
                              target.src = feelsGuyImage.src;
                            }}
                          />
                        </div>
                        
                        {/* Token Info */}
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center justify-between">
                            <div>
                              <h3 className="font-semibold truncate">
                                {token.name}
                                <span className="text-muted-foreground ml-2">{token.symbol}</span>
                              </h3>
                              <p className="text-xs text-muted-foreground">
                                {token.address.slice(0, 8)}...{token.address.slice(-8)}
                              </p>
                            </div>
                            <div className="flex flex-col items-end gap-1">
                              <span className="text-sm font-medium">{token.marketCap}</span>
                              <Badge variant="outline" className="text-xs">
                                {token.launched}
                              </Badge>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
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