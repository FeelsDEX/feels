'use client';

import { Connection } from '@solana/web3.js';
import { Program, Idl } from '@coral-xyz/anchor';
import { useProtocolStats } from '@/hooks/useIndexer';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { TrendingUp, TrendingDown, Activity, DollarSign, Percent, Clock, Zap, AlertCircle, CheckCircle, Shield } from 'lucide-react';

interface FeelsMetricsProps {
  program: Program<Idl> | null;
  connection: Connection;
}

interface Metric {
  label: string;
  value: string | number;
  feelsValue?: number; // SOL amount
  change?: number;
  icon?: React.ReactNode;
  format?: 'number' | 'currency' | 'percent' | 'time' | 'dual' | 'text';
}

export function FeelsMetrics({}: FeelsMetricsProps) {
  const indexerStats = useProtocolStats({
    refreshInterval: 10000, // Refresh every 10 seconds
  });

  // Mock SOL price for conversion (in production, get from oracle)
  const solPrice = 50; // $50 per SOL

  // Calculate 16 key metrics based on protocol docs
  // Note: Some metrics are estimated or mocked until the indexer provides them
  const totalLiquidity = parseFloat(indexerStats.data?.total_liquidity || '0') / 1e9; // Convert from lamports to SOL
  
  const metrics: Metric[] = [
    // Protocol Health Metrics (Critical)
    {
      label: 'Solvency Ratio',
      value: 102.5, // JitoSOL reserves / SOL supply (>100% due to staking yield)
      format: 'percent',
      icon: <Shield className="h-4 w-4 text-primary" />,
    },
    {
      label: 'Oracle Status',
      value: 'Healthy', // Protocol oracle health
      format: 'text',
      icon: <CheckCircle className="h-4 w-4 text-primary" />,
    },
    {
      label: 'JitoSOL/SOL',
      value: 1.035, // Current exchange rate
      format: 'number',
      icon: <Activity className="h-4 w-4" />,
    },
    {
      label: 'Safety Status',
      value: 'Normal', // Safety controller status
      format: 'text',
      icon: <CheckCircle className="h-4 w-4 text-primary" />,
    },

    // Backing & Reserves
    {
      label: 'JitoSOL Reserves',
      value: totalLiquidity * 0.6 * solPrice, // Estimated 60% of liquidity is JitoSOL reserves
      feelsValue: totalLiquidity * 0.6,
      format: 'dual',
      icon: <DollarSign className="h-4 w-4" />,
    },
    {
      label: 'SOL Supply',
      value: totalLiquidity * solPrice, // Total liquidity approximates SOL supply
      feelsValue: totalLiquidity,
      format: 'dual',
    },
    {
      label: 'Staking Yield APY',
      value: 7.2, // Current JitoSOL APY
      format: 'percent',
      icon: <TrendingUp className="h-4 w-4 text-primary" />,
    },
    {
      label: 'Pools at Floor',
      value: 2, // Number of pools trading near floor
      format: 'number',
      icon: <AlertCircle className="h-4 w-4 text-yellow-500" />,
    },

    // Market Activity
    {
      label: 'Active Markets',
      value: indexerStats.data?.total_markets || 0,
      format: 'number',
      icon: <Activity className="h-4 w-4" />,
    },
    {
      label: '24h Volume',
      value: (indexerStats.data?.total_volume_24h || 0) * solPrice,
      feelsValue: indexerStats.data?.total_volume_24h || 0,
      format: 'dual',
    },
    {
      label: '24h Fees',
      value: (indexerStats.data?.total_fees_24h || 0) * solPrice,
      feelsValue: indexerStats.data?.total_fees_24h || 0,
      format: 'dual',
      icon: <Percent className="h-4 w-4" />,
    },
    {
      label: 'Volatility',
      value: 'Normal', // Ticks/second indicator
      format: 'text',
      icon: <Activity className="h-4 w-4 text-primary" />,
    },

    // Liquidity & Positions
    {
      label: 'Total Liquidity',
      value: totalLiquidity * solPrice,
      feelsValue: totalLiquidity,
      format: 'dual',
      icon: <Zap className="h-4 w-4" />,
    },
    {
      label: 'Floor Liquidity',
      value: totalLiquidity * 0.1 * solPrice, // Estimated 10% at floor
      feelsValue: totalLiquidity * 0.1,
      format: 'dual',
    },
    {
      label: 'Floor Updates',
      value: '15m ago', // Last floor ratchet
      format: 'text',
      icon: <Clock className="h-4 w-4" />,
    },
    {
      label: 'GTWAP Status',
      value: 8, // Number of markets with healthy GTWAP
      format: 'number',
      icon: <CheckCircle className="h-4 w-4 text-primary" />,
    },
  ];

  const formatValue = (metric: Metric): string | React.ReactNode => {
    const value = metric.value;
    
    switch (metric.format) {
      case 'currency':
        if (typeof value === 'number') {
          if (value >= 1000000) {
            return `$${(value / 1000000).toFixed(2)}M`;
          } else if (value >= 1000) {
            return `$${(value / 1000).toFixed(1)}K`;
          }
          return `$${value.toFixed(2)}`;
        }
        return String(value);
      
      case 'percent':
        if (typeof value === 'number') {
          return `${value.toFixed(2)}%`;
        }
        return String(value);
      
      case 'number':
        if (typeof value === 'number') {
          return value.toLocaleString();
        }
        return String(value);
      
      case 'dual':
        if (typeof value === 'number' && metric.feelsValue !== undefined) {
          const dollarValue = value >= 1000000 
            ? `$${(value / 1000000).toFixed(2)}M`
            : value >= 1000
            ? `$${(value / 1000).toFixed(1)}K`
            : `$${value.toFixed(2)}`;
          
          const feelsAmount = metric.feelsValue >= 1000000
            ? `${(metric.feelsValue / 1000000).toFixed(2)}M`
            : metric.feelsValue >= 1000
            ? `${(metric.feelsValue / 1000).toFixed(1)}K`
            : metric.feelsValue.toFixed(2);
          
          return (
            <div className="space-y-0.5">
              <div className="text-lg font-semibold">{dollarValue}</div>
              <div className="text-xs text-muted-foreground">{feelsAmount} ◈</div>
            </div>
          );
        }
        return String(value);
      
      case 'text':
        return String(value);
      
      default:
        return String(value);
    }
  };

  if (indexerStats.loading) {
    return (
      <Card className="w-full">
        <CardHeader>
          <CardTitle className="text-xl">Feels Metrics</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            {[...Array(16)].map((_, i) => (
              <div key={i} className="animate-pulse">
                <div className="h-4 bg-muted rounded w-2/3 mb-2"></div>
                <div className="h-6 bg-muted rounded w-full"></div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    );
  }

  if (indexerStats.error) {
    return (
      <Card className="w-full">
        <CardHeader>
          <CardTitle className="text-xl">Feels Metrics</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-destructive text-sm">
            Error loading metrics: {indexerStats.error}
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="w-full">
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-xl">Feels Metrics</CardTitle>
          {indexerStats.lastUpdated && (
            <div className="flex items-center text-xs text-muted-foreground">
              <Clock className="h-3 w-3 mr-1" />
              {new Date(indexerStats.lastUpdated).toLocaleTimeString()}
            </div>
          )}
        </div>
      </CardHeader>
      <CardContent>
        <div className="space-y-6">
          {/* Protocol Health Section */}
          <div>
            <h3 className="text-sm font-medium text-muted-foreground mb-3">Protocol Health</h3>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
              {metrics.slice(0, 4).map((metric, index) => (
                <div
                  key={index}
                  className="p-3 rounded-lg border bg-card"
                >
                  <div className="flex items-center justify-between mb-1">
                    <p className="text-xs text-muted-foreground">{metric.label}</p>
                    {metric.icon}
                  </div>
                  <div>
                    {formatValue(metric)}
                  </div>
                  {metric.change !== undefined && (
                    <div className="flex items-center mt-1">
                      {metric.change >= 0 ? (
                        <TrendingUp className="h-3 w-3 text-primary mr-1" />
                      ) : (
                        <TrendingDown className="h-3 w-3 text-danger-500 mr-1" />
                      )}
                      <span className={`text-xs ${metric.change >= 0 ? 'text-primary' : 'text-danger-500'}`}>
                        {Math.abs(metric.change).toFixed(1)}%
                      </span>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>

          {/* Backing & Reserves Section */}
          <div>
            <h3 className="text-sm font-medium text-muted-foreground mb-3">Backing & Reserves</h3>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
              {metrics.slice(4, 8).map((metric, index) => (
                <div
                  key={index + 4}
                  className="p-3 rounded-lg border bg-card"
                >
                  <div className="flex items-center justify-between mb-1">
                    <p className="text-xs text-muted-foreground">{metric.label}</p>
                    {metric.icon}
                  </div>
                  <div>
                    {formatValue(metric)}
                  </div>
                  {metric.change !== undefined && (
                    <div className="flex items-center mt-1">
                      {metric.change >= 0 ? (
                        <TrendingUp className="h-3 w-3 text-primary mr-1" />
                      ) : (
                        <TrendingDown className="h-3 w-3 text-danger-500 mr-1" />
                      )}
                      <span className={`text-xs ${metric.change >= 0 ? 'text-primary' : 'text-danger-500'}`}>
                        {Math.abs(metric.change).toFixed(1)}%
                      </span>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>

          {/* Market Activity Section */}
          <div>
            <h3 className="text-sm font-medium text-muted-foreground mb-3">Market Activity</h3>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
              {metrics.slice(8, 12).map((metric, index) => (
                <div
                  key={index + 8}
                  className="p-3 rounded-lg border bg-card"
                >
                  <div className="flex items-center justify-between mb-1">
                    <p className="text-xs text-muted-foreground">{metric.label}</p>
                    {metric.icon}
                  </div>
                  <div>
                    {formatValue(metric)}
                  </div>
                  {metric.change !== undefined && (
                    <div className="flex items-center mt-1">
                      {metric.change >= 0 ? (
                        <TrendingUp className="h-3 w-3 text-primary mr-1" />
                      ) : (
                        <TrendingDown className="h-3 w-3 text-danger-500 mr-1" />
                      )}
                      <span className={`text-xs ${metric.change >= 0 ? 'text-primary' : 'text-danger-500'}`}>
                        {Math.abs(metric.change).toFixed(1)}%
                      </span>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>

          {/* Liquidity & Positions Section */}
          <div>
            <h3 className="text-sm font-medium text-muted-foreground mb-3">Liquidity & Positions</h3>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
              {metrics.slice(12, 16).map((metric, index) => (
                <div
                  key={index + 12}
                  className="p-3 rounded-lg border bg-card"
                >
                  <div className="flex items-center justify-between mb-1">
                    <p className="text-xs text-muted-foreground">{metric.label}</p>
                    {metric.icon}
                  </div>
                  <div>
                    {formatValue(metric)}
                  </div>
                  {metric.change !== undefined && (
                    <div className="flex items-center mt-1">
                      {metric.change >= 0 ? (
                        <TrendingUp className="h-3 w-3 text-primary mr-1" />
                      ) : (
                        <TrendingDown className="h-3 w-3 text-danger-500 mr-1" />
                      )}
                      <span className={`text-xs ${metric.change >= 0 ? 'text-primary' : 'text-danger-500'}`}>
                        {Math.abs(metric.change).toFixed(1)}%
                      </span>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}