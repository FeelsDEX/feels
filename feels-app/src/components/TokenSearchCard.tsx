'use client';

import React from 'react';
import { TokenSearchResult } from '@/lib/token-search';
import { Badge } from '@/components/ui/badge';
import { 
  TrendingUp, 
  TrendingDown, 
  Activity, 
  DollarSign,
  Clock,
  Shield,
  Droplet,
  Trophy
} from 'lucide-react';
import Link from 'next/link';
import Image from 'next/image';
import feelsGuyImage from '@/assets/images/feels_guy.png';

interface TokenSearchCardProps {
  token: TokenSearchResult;
}

export const TokenSearchCard = React.memo(function TokenSearchCard({ token }: TokenSearchCardProps) {
  const priceChangeColor = token.priceChange24h >= 0 ? 'text-primary' : 'text-red-500';
  const priceChangeIcon = token.priceChange24h >= 0 ? TrendingUp : TrendingDown;
  const PriceIcon = priceChangeIcon;
  
  return (
    <Link href={`/token/${token.address}`} className="block">
      <div className="border rounded-lg p-4 hover:shadow-md hover:border-primary/50 transition-all cursor-pointer">
        <div className="flex items-start gap-4">
          {/* Token Image */}
          <div className="relative h-16 w-16 flex-shrink-0">
            <Image
              src={token.imageUrl}
              alt={token.name}
              fill
              sizes="64px"
              className="rounded-full object-cover"
              onError={(e) => {
                const target = e.target as HTMLImageElement;
                target.src = feelsGuyImage.src;
              }}
            />
          </div>
          
          {/* Token Info */}
          <div className="flex-1 min-w-0">
            <div className="flex items-start justify-between gap-4 mb-2">
              <div>
                <h3 className="font-semibold text-lg truncate">
                  {token.name}
                  <span className="text-muted-foreground ml-2">{token.symbol}</span>
                </h3>
                <p className="text-xs text-muted-foreground truncate">
                  {token.address.slice(0, 8)}...{token.address.slice(-8)}
                </p>
              </div>
              
              {/* Features */}
              <div className="flex items-center gap-1">
                {token.isVerified && (
                  <Badge variant="secondary" className="text-xs">
                    <Shield className="h-3 w-3 mr-1" />
                    Verified
                  </Badge>
                )}
                {token.isGraduated && (
                  <Badge variant="secondary" className="text-xs">
                    <Trophy className="h-3 w-3 mr-1" />
                    Graduated
                  </Badge>
                )}
                {token.hasLiquidity && (
                  <Badge variant="secondary" className="text-xs">
                    <Droplet className="h-3 w-3 mr-1" />
                    Liquidity
                  </Badge>
                )}
              </div>
            </div>
            
            {/* Description */}
            {token.description && (
              <p className="text-sm text-muted-foreground mb-3 line-clamp-2">
                {token.description}
              </p>
            )}
            
            {/* Metrics */}
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-3">
              <div>
                <div className="flex items-center gap-1 text-xs text-muted-foreground mb-1">
                  <DollarSign className="h-3 w-3" />
                  Market Cap
                </div>
                <p className="font-semibold">{token.marketCapFormatted}</p>
              </div>
              
              <div>
                <div className="flex items-center gap-1 text-xs text-muted-foreground mb-1">
                  <Activity className="h-3 w-3" />
                  24h Volume
                </div>
                <p className="font-semibold">{token.volume24hFormatted}</p>
              </div>
              
              <div>
                <div className="flex items-center gap-1 text-xs text-muted-foreground mb-1">
                  <PriceIcon className="h-3 w-3" />
                  24h Change
                </div>
                <p className={`font-semibold ${priceChangeColor}`}>
                  {token.priceChange24h > 0 ? '+' : ''}{token.priceChange24h.toFixed(2)}%
                </p>
              </div>
              
              <div>
                <div className="flex items-center gap-1 text-xs text-muted-foreground mb-1">
                  <Clock className="h-3 w-3" />
                  Age
                </div>
                <p className="font-semibold">{token.launched}</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Link>
  );
});