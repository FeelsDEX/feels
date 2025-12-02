// Compact token card component for grid view on the splash page
'use client';

import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import Link from 'next/link';
import Image from 'next/image';
import { TokenSearchResult } from '@/utils/token-search';
import feelsGuyImage from '@/assets/images/feels_guy.png';

interface CompactTokenCardProps {
  token: TokenSearchResult;
}

function formatMetricValue(value: number): string {
  if (value >= 1) return value.toFixed(4);
  else if (value >= 0.01) return value.toFixed(4);
  else if (value >= 0.001) return value.toFixed(5);
  else return value.toFixed(6);
}

function formatLargeNumber(num: number): string {
  if (num >= 1_000_000_000) {
    return `$${(num / 1_000_000_000).toFixed(2)}B`;
  } else if (num >= 1_000_000) {
    return `$${(num / 1_000_000).toFixed(2)}M`;
  } else if (num >= 1_000) {
    return `$${(num / 1_000).toFixed(2)}K`;
  }
  return `$${num.toFixed(2)}`;
}

export function CompactTokenCard({ token }: CompactTokenCardProps) {
  return (
    <Link 
      href={`/token/${token.address}`} 
      className="block group"
    >
      <Card 
        className="h-full hover:shadow-lg hover:border-primary transition-all cursor-pointer overflow-hidden border"
      >
        <div className="flex flex-col h-full">
          {/* Image Section */}
          <div className="aspect-square bg-white flex items-center justify-center p-3">
            <div className="relative w-full h-full">
              {typeof token.imageUrl === 'string' ? (
                <img 
                  src={token.imageUrl} 
                  alt={token.name}
                  className="w-full h-full object-contain rounded-lg"
                  loading="lazy"
                />
              ) : (
                <Image 
                  src={token.imageUrl || feelsGuyImage} 
                  alt={token.name}
                  fill
                  className="object-contain rounded-lg"
                  sizes="(max-width: 768px) 100vw, (max-width: 1200px) 33vw, 25vw"
                />
              )}
            </div>
          </div>
          
          {/* Content Section */}
          <div className="flex-1 flex flex-col p-3">
            {/* Header */}
            <div className="flex items-start justify-between mb-3">
              <div className="flex-1 min-w-0">
                <h3 className="text-sm font-bold text-foreground leading-tight truncate">
                  {token.name}
                </h3>
                <p className="text-xs text-muted-foreground truncate">${token.symbol}</p>
              </div>
              <div className="flex items-center gap-1 shrink-0 ml-2">
                {token.isTrending && (
                  <Badge 
                    variant="default" 
                    className="text-xs px-1.5 py-0 h-5 bg-primary/10 text-primary border-primary/20 hover:bg-primary/10 hover:text-primary"
                  >
                    Trending
                  </Badge>
                )}
                {token.isVerified && (
                  <Badge 
                    variant="outline" 
                    className="text-xs px-1.5 py-0 h-5"
                  >
                    ✓
                  </Badge>
                )}
              </div>
            </div>

            {/* Stats Grid */}
            <div className="grid grid-cols-2 gap-x-2 gap-y-2 text-xs">
              {/* Price */}
              <div>
                <span className="text-muted-foreground block mb-0.5">Price</span>
                <span className="font-semibold text-foreground">${formatMetricValue(token.price)}</span>
              </div>
              
              {/* 24h Change */}
              <div>
                <span className="text-muted-foreground block mb-0.5">24h</span>
                <span 
                  className={`font-semibold ${token.priceChange24h >= 0 ? 'text-primary' : 'text-danger-500'}`}
                >
                  {token.priceChange24h >= 0 ? '+' : ''}{token.priceChange24h.toFixed(2)}%
                </span>
              </div>
              
              {/* Market Cap */}
              <div>
                <span className="text-muted-foreground block mb-0.5">MCap</span>
                <span className="font-semibold text-foreground">{formatLargeNumber(token.marketCap)}</span>
              </div>
              
              {/* Volume */}
              <div>
                <span className="text-muted-foreground block mb-0.5">Vol</span>
                <span className="font-semibold text-foreground">{formatLargeNumber(token.volume24h)}</span>
              </div>
            </div>
          </div>
        </div>
      </Card>
    </Link>
  );
}

