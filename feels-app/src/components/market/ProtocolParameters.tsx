'use client';

import { useState, useEffect } from 'react';
import { Connection } from '@solana/web3.js';
import { Program, Idl } from '@coral-xyz/anchor';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Info, Settings, Shield, Percent } from 'lucide-react';

interface ProtocolParametersProps {
  program: Program<Idl> | null;
  connection: Connection;
}

interface ParameterGroup {
  title: string;
  icon: React.ReactNode;
  parameters: Parameter[];
}

interface Parameter {
  name: string;
  value: string | number | boolean;
  unit?: string;
  description?: string;
  type?: 'fee' | 'time' | 'feature' | 'safety';
}

export function ProtocolParameters({}: ProtocolParametersProps) {
  const [loading, setLoading] = useState(true);
  
  // Mock protocol parameters based on documentation
  const parameterGroups: ParameterGroup[] = [
    {
      title: 'Fee Configuration',
      icon: <Percent className="h-4 w-4" />,
      parameters: [
        {
          name: 'Base Fee',
          value: 0.30,
          unit: '%',
          description: 'Standard trading fee',
          type: 'fee',
        },
        {
          name: 'Min Total Fee',
          value: 0.20,
          unit: '%',
          description: 'Minimum fee cap',
          type: 'fee',
        },
        {
          name: 'Max Total Fee',
          value: 1.50,
          unit: '%',
          description: 'Maximum fee cap',
          type: 'fee',
        },
        {
          name: 'Impact Floor',
          value: 0.10,
          unit: '%',
          description: 'Minimum price impact fee',
          type: 'fee',
        },
      ],
    },
    {
      title: 'Fee Distribution',
      icon: <Settings className="h-4 w-4" />,
      parameters: [
        {
          name: 'LP Share',
          value: 45,
          unit: '%',
          description: 'Liquidity provider rewards',
          type: 'fee',
        },
        {
          name: 'Pool Reserve',
          value: 25,
          unit: '%',
          description: 'Pool stability fund',
          type: 'fee',
        },
        {
          name: 'Pool Buffer',
          value: 20,
          unit: '%',
          description: 'JIT liquidity fund',
          type: 'fee',
        },
        {
          name: 'Protocol Treasury',
          value: 8,
          unit: '%',
          description: 'Protocol development',
          type: 'fee',
        },
        {
          name: 'Creator',
          value: 2,
          unit: '%',
          description: 'Token creator share',
          type: 'fee',
        },
      ],
    },
    {
      title: 'Safety Parameters',
      icon: <Shield className="h-4 w-4" />,
      parameters: [
        {
          name: 'Floor Buffer',
          value: 100,
          unit: 'ticks',
          description: 'Price floor protection margin',
          type: 'safety',
        },
        {
          name: 'Ratchet Cooldown',
          value: 1800,
          unit: 'slots',
          description: '~15 minutes cooldown',
          type: 'time',
        },
        {
          name: 'Oracle Safety Buffer',
          value: 0.50,
          unit: '%',
          description: 'Oracle price deviation tolerance',
          type: 'safety',
        },
        {
          name: 'Min Warmup Slots',
          value: 2400,
          unit: 'slots',
          description: '~20 minutes before dynamic fees',
          type: 'time',
        },
        {
          name: 'Min Warmup Trades',
          value: 150,
          unit: 'trades',
          description: 'Minimum trades before rebates',
          type: 'safety',
        },
      ],
    },
    {
      title: 'JIT Configuration',
      icon: <Info className="h-4 w-4" />,
      parameters: [
        {
          name: 'JIT v0.5 Active',
          value: true,
          description: 'Virtual concentrated liquidity',
          type: 'feature',
        },
        {
          name: 'Base Spread',
          value: 3,
          unit: 'ticks',
          description: 'JIT quote spread',
          type: 'safety',
        },
        {
          name: 'Per Swap Budget',
          value: 3.0,
          unit: '%',
          description: 'Max buffer per swap',
          type: 'safety',
        },
        {
          name: 'Per Slot Budget',
          value: 5.0,
          unit: '%',
          description: 'Max buffer per slot',
          type: 'safety',
        },
        {
          name: 'Concentration Boost',
          value: '10x',
          description: 'Max liquidity concentration',
          type: 'feature',
        },
      ],
    },
  ];

  useEffect(() => {
    // Simulate loading
    const timer = setTimeout(() => {
      setLoading(false);
    }, 500);
    return () => clearTimeout(timer);
  }, []);

  if (loading) {
    return (
      <Card className="w-full">
        <CardHeader>
          <CardTitle className="text-xl">Protocol Parameters</CardTitle>
          <CardDescription>Current protocol configuration and governance settings</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-6">
            {[...Array(4)].map((_, i) => (
              <div key={i} className="animate-pulse">
                <div className="h-5 bg-muted rounded w-1/3 mb-3"></div>
                <div className="space-y-2">
                  {[...Array(4)].map((_, j) => (
                    <div key={j} className="h-4 bg-muted rounded w-full"></div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    );
  }

  const getBadgeVariant = (type?: string): "default" | "secondary" | "destructive" | "outline" => {
    switch (type) {
      case 'fee':
        return 'secondary';
      case 'safety':
        return 'destructive';
      case 'feature':
        return 'default';
      default:
        return 'outline';
    }
  };

  return (
    <Card className="w-full">
      <CardHeader>
        <CardTitle className="text-xl">Protocol Parameters</CardTitle>
        <CardDescription>Current protocol configuration and governance settings</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-6">
          {parameterGroups.map((group, groupIndex) => (
            <div key={groupIndex}>
              <div className="flex items-center gap-2 mb-3">
                {group.icon}
                <h3 className="font-medium">{group.title}</h3>
              </div>
              <div className="space-y-2">
                {group.parameters.map((param, paramIndex) => (
                  <div
                    key={paramIndex}
                    className="flex items-center justify-between p-2 rounded-lg hover:bg-muted/50 transition-colors"
                  >
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium">{param.name}</span>
                        {param.type && (
                          <Badge variant={getBadgeVariant(param.type)} className="text-xs h-5">
                            {param.type}
                          </Badge>
                        )}
                      </div>
                      {param.description && (
                        <p className="text-xs text-muted-foreground mt-0.5">
                          {param.description}
                        </p>
                      )}
                    </div>
                    <div className="text-right">
                      <span className="text-sm font-mono">
                        {typeof param.value === 'boolean' ? (
                          <Badge variant={param.value ? 'default' : 'secondary'}>
                            {param.value ? 'Enabled' : 'Disabled'}
                          </Badge>
                        ) : (
                          <>
                            {param.value}
                            {param.unit && ` ${param.unit}`}
                          </>
                        )}
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
        
        <div className="mt-6 p-4 bg-warning/10 border border-warning/20 rounded-lg">
          <div className="flex items-start gap-2">
            <span className="text-warning text-sm font-bold">WARNING</span>
            <div>
              <p className="text-sm font-medium">Test Environment</p>
              <p className="text-xs text-muted-foreground mt-0.5">
                These parameters are for the Devnet deployment. Production values may differ based on governance decisions.
              </p>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}