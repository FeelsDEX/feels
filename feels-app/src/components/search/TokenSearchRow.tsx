'use client';

import React from 'react';
import { TokenSearchResult } from '@/utils/token-search';
import { Badge } from '@/components/ui/badge';
import { TrendingUp, TrendingDown } from 'lucide-react';
import Link from 'next/link';
import Image from 'next/image';
import feelsGuyImage from '@/assets/images/feels_guy.png';

interface TokenSearchRowProps {
  token: TokenSearchResult;
}

export const TokenSearchRow = React.memo(function TokenSearchRow({ token }: TokenSearchRowProps) {
  const priceChangeColor = token.priceChange24h >= 0 ? 'text-primary' : 'text-red-500';
  const PriceIcon = token.priceChange24h >= 0 ? TrendingUp : TrendingDown;
  
  return (
    <Link href={`/token/${token.address}`} className="block">
      <div className="flex items-center gap-4 px-4 py-3 hover:bg-muted/50 transition-colors cursor-pointer">
        {/* Token Image */}
        <div className="w-10 h-10 relative flex-shrink-0">
          <Image
            src={token.imageUrl}
            alt={token.name}
            fill
            sizes="40px"
            className="rounded-md object-cover"
            onError={(e) => {
              const target = e.target as HTMLImageElement;
              target.src = feelsGuyImage.src;
            }}
          />
        </div>
        
        {/* Name & Symbol */}
        <div className="flex-1 min-w-[200px]">
          <div className="flex items-center gap-2">
            <div>
              <div className="font-medium">{token.name}</div>
              <div className="text-sm text-muted-foreground">{token.symbol}</div>
            </div>
            {token.isGraduated && (
              <Badge variant="secondary" className="text-xs self-start">
                Graduated
              </Badge>
            )}
          </div>
        </div>
        
        {/* Market Cap */}
        <div className="w-24 text-right">
          <div className="text-sm font-medium">{token.marketCapFormatted}</div>
        </div>
        
        {/* 24h Volume */}
        <div className="w-24 text-right">
          <div className="text-sm font-medium">{token.volume24hFormatted}</div>
        </div>
        
        {/* Price (as 24h Range placeholder) */}
        <div className="w-28 text-right">
          <div className="text-sm font-medium">${token.price.toFixed(4)}</div>
        </div>
        
        {/* Price/Change (as Floor/GTWAP placeholder) */}
        <div className="w-36 text-right">
          <div className="flex items-baseline justify-end gap-0.5">
            <span className="text-sm font-medium">${token.price.toFixed(2)}</span>
            <span className="text-xs font-semibold text-muted-foreground">
              ({token.priceChange24h > 0 ? '+' : ''}{token.priceChange24h.toFixed(0)}%)
            </span>
          </div>
        </div>
        
        {/* 24h Change (as Floor Δ 24h) */}
        <div className="w-24 text-right">
          <div className={`text-sm font-medium ${priceChangeColor}`}>
            {token.priceChange24h >= 0 ? '+' : ''}{token.priceChange24h.toFixed(2)}%
          </div>
        </div>
      </div>
    </Link>
  );
});