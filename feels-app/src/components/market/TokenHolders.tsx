'use client';

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import Link from 'next/link';
import { Users } from 'lucide-react';

interface TokenHolder {
  address: string;
  percentage: number;
  isCreator?: boolean;
}

interface TokenHoldersProps {
  tokenAddress: string;
  tokenCreator?: string;
}

export function TokenHolders({ tokenCreator }: TokenHoldersProps) {
  // Mock data - in production this would come from the indexer/API
  // Generate mock holders with realistic distribution
  const generateMockHolders = (): TokenHolder[] => {
    const holders: TokenHolder[] = [];
    
    // Creator typically holds 5-20% in memecoins
    if (tokenCreator) {
      holders.push({
        address: tokenCreator,
        percentage: 15.8,
        isCreator: true
      });
    }
    
    // Top holders with decreasing percentages
    const topHolderAddresses = [
      'H5PM3UTgejrBHFfKEAY2cGdsFnTuFvEfJiNU8X7FdDRB',
      '3Xny6SaKZfUZRzHWkf9kg3HYkPJqq6vwEFkVMVa3miKJ',
      'DYw8jCTfwHNRJhhmFcbXvVDTqWMEVFBX6ZKUmG5CNSKK',
      '9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM',
      'FriELggez2Dy3phZeHHAdpcoEXkKQVkv6tx3zDtCVP8T',
      '2g9TLFJDP3iKqPUuBa5pHFECyNvQHz7HUVJhfGLNdQPi',
      '7oo1B5YcKJGBH2sV7qrFMPRQ5zM9sZrQ8fRxbM6VBJd8',
      'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL'
    ];
    
    // Distribute remaining percentage among top holders
    let remainingPercentage = tokenCreator ? 84.2 : 100;
    const percentages = [8.5, 6.2, 5.1, 4.8, 3.9, 3.2, 2.7, 2.4];
    
    topHolderAddresses.forEach((address, index) => {
      if (address !== tokenCreator && index < percentages.length) {
        const percentage = percentages[index];
        if (percentage !== undefined) {
          holders.push({
            address,
            percentage
          });
          remainingPercentage -= percentage;
        }
      }
    });
    
    // Add more smaller holders to reach exactly 10
    const smallerHolders = [
      { address: 'Esmx2QjmDZMjJ15yBJ3nhDLq7mFBxfbVd8zEvcYBHWWh', percentage: 2.1 },
      { address: 'Cv4gPFX8ycN6svcymmVg8bPULYBHCJpRZnazMPQTPfGu', percentage: 1.8 }
    ];
    
    holders.push(...smallerHolders.filter(h => h.address !== tokenCreator));
    
    // Sort by percentage descending and limit to top 10
    return holders.sort((a, b) => b.percentage - a.percentage).slice(0, 10);
  };
  
  const holders = generateMockHolders();
  const totalPercentageShown = holders.reduce((sum, holder) => sum + holder.percentage, 0);
  
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Users className="h-5 w-5" />
          Holders
        </CardTitle>
      </CardHeader>
      <CardContent className="pb-3">
        <div className="space-y-2">
          {/* Column Headers */}
          <div className="grid grid-cols-[1fr,auto] gap-4 pb-1 border-b border-border text-sm text-muted-foreground">
            <div>Account</div>
            <div>Ownership</div>
          </div>
          
          {/* Holder List */}
          <div className="space-y-1">
            {holders.map((holder, index) => (
              <div
                key={holder.address}
                className="grid grid-cols-[1fr,auto] gap-4 py-1 items-center hover:bg-muted/30 rounded-lg px-2 -mx-2 transition-colors"
              >
                <div className="flex items-center gap-2 min-w-0">
                  <span className="text-xs text-muted-foreground">
                    {(index + 1).toString().padStart(2, '0')}
                  </span>
                  <Link
                    href={`/account/${holder.address}`}
                    className="text-sm font-mono truncate text-muted-foreground hover:text-primary transition-colors ml-1"
                    title={holder.address}
                  >
                    {holder.address.slice(0, 4)}...{holder.address.slice(-4)}
                  </Link>
                  {holder.isCreator && (
                    <Badge 
                      variant="outline" 
                      className="text-xs px-1.5 py-0 h-5 bg-primary/10 text-primary border-primary/20 cursor-default"
                    >
                      Dev
                    </Badge>
                  )}
                </div>
                <div className="text-sm font-medium">
                  {holder.percentage.toFixed(1)}%
                </div>
              </div>
            ))}
          </div>
          
          {/* Summary */}
          <div className="pt-1.5 mt-1.5 pb-3 border-t border-border">
            <div className="flex justify-between text-sm text-muted-foreground">
              <span>Others</span>
              <span>{(100 - totalPercentageShown).toFixed(1)}%</span>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}