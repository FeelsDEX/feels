'use client';

import { useState, useEffect } from 'react';
import { Connection, PublicKey } from '@solana/web3.js';
import { Program, Idl, BN } from '@coral-xyz/anchor';
import { useWallet } from '@solana/wallet-adapter-react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { toast } from '@/hooks/use-toast';
import { Info, Settings, Shield, Percent, Loader2, AlertCircle, Lock, Unlock } from 'lucide-react';

interface ProtocolParametersAdminProps {
  program: Program<Idl> | null;
  connection: Connection;
  fallback?: boolean;
}

interface Parameter {
  name: string;
  field: string;
  value: string | number | boolean;
  unit?: string;
  description?: string;
  type?: 'fee' | 'time' | 'feature' | 'safety' | 'admin';
  min?: number;
  max?: number;
  decimals?: number;
}

// PDAs for protocol accounts
const PROTOCOL_CONFIG_SEED = Buffer.from('protocol_config');
// const PROTOCOL_ORACLE_SEED = Buffer.from('protocol_oracle'); // Commented out as unused
// const SAFETY_CONTROLLER_SEED = Buffer.from('safety_controller'); // Commented out as unused

export function ProtocolParametersAdmin({ program, connection, fallback = false }: ProtocolParametersAdminProps) {
  const { publicKey } = useWallet();
  const [loading, setLoading] = useState(true);
  const [isAuthority, setIsAuthority] = useState(false);
  const [protocolConfig, setProtocolConfig] = useState<any>(null);
  const [editMode, setEditMode] = useState(false);
  const [pendingChanges, setPendingChanges] = useState<Record<string, any>>({});
  const [submitting, setSubmitting] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [accountName, setAccountName] = useState<string>('protocolConfig');

  // Fee Configuration parameters
  const feeParameters: Parameter[] = [
    {
      name: 'Mint Fee',
      field: 'mint_fee',
      value: protocolConfig?.mintFee ? Number(protocolConfig.mintFee) / 1e9 : 0.001,
      unit: 'SOL',
      description: 'Fee for minting SOL',
      type: 'fee',
      min: 0,
      max: 1,
      decimals: 4,
    },
    {
      name: 'Protocol Fee Rate',
      field: 'default_protocol_fee_rate',
      value: protocolConfig?.defaultProtocolFeeRate || 8,
      unit: 'bps',
      description: 'Default protocol fee rate',
      type: 'fee',
      min: 0,
      max: 100,
    },
    {
      name: 'Creator Fee Rate',
      field: 'default_creator_fee_rate',
      value: protocolConfig?.defaultCreatorFeeRate || 2,
      unit: 'bps',
      description: 'Default creator fee rate',
      type: 'fee',
      min: 0,
      max: 100,
    },
    {
      name: 'Max Protocol Fee',
      field: 'max_protocol_fee_rate',
      value: protocolConfig?.maxProtocolFeeRate || 50,
      unit: 'bps',
      description: 'Maximum protocol fee cap',
      type: 'fee',
      min: 10,
      max: 200,
    },
  ];

  // Safety & Circuit Breaker parameters
  const safetyParameters: Parameter[] = [
    {
      name: 'Depeg Threshold',
      field: 'depeg_threshold_bps',
      value: protocolConfig?.depegThresholdBps || 50,
      unit: 'bps',
      description: 'Oracle price deviation tolerance to trigger circuit breaker',
      type: 'safety',
      min: 25,
      max: 5000,
    },
    {
      name: 'Depeg Required Obs',
      field: 'depeg_required_obs',
      value: protocolConfig?.depegRequiredObs || 3,
      unit: 'obs',
      description: 'Consecutive observations to trigger depeg and pause redemptions',
      type: 'safety',
      min: 1,
      max: 10,
    },
    {
      name: 'Clear Required Obs',
      field: 'clear_required_obs',
      value: protocolConfig?.clearRequiredObs || 5,
      unit: 'obs',
      description: 'Consecutive observations to clear depeg and resume redemptions',
      type: 'safety',
      min: 1,
      max: 10,
    },
    {
      name: 'DEX TWAP Window',
      field: 'dex_twap_window_secs',
      value: protocolConfig?.dexTwapWindowSecs || 900,
      unit: 'seconds',
      description: 'Time window for DEX TWAP price calculation',
      type: 'time',
      min: 300,
      max: 7200,
    },
    {
      name: 'DEX TWAP Stale Age',
      field: 'dex_twap_stale_age_secs',
      value: protocolConfig?.dexTwapStaleAgeSecs || 1800,
      unit: 'seconds',
      description: 'Maximum age before DEX TWAP is considered stale',
      type: 'time',
      min: 300,
      max: 7200,
    },
    {
      name: 'Mint Per Slot Cap',
      field: 'mint_per_slot_cap_feelssol',
      value: protocolConfig?.mintPerSlotCapFeelssol ? Number(protocolConfig.mintPerSlotCapFeelssol) / 1e9 : 0,
      unit: 'SOL',
      description: 'Max SOL mints per slot (0 = unlimited)',
      type: 'safety',
      min: 0,
      max: 10000,
      decimals: 2,
    },
    {
      name: 'Redeem Per Slot Cap',
      field: 'redeem_per_slot_cap_feelssol',
      value: protocolConfig?.redeemPerSlotCapFeelssol ? Number(protocolConfig.redeemPerSlotCapFeelssol) / 1e9 : 0,
      unit: 'SOL',
      description: 'Max SOL redemptions per slot (0 = unlimited)',
      type: 'safety',
      min: 0,
      max: 10000,
      decimals: 2,
    },
  ];

  // Administrative parameters
  const adminParameters: Parameter[] = [
    {
      name: 'Treasury Account',
      field: 'treasury',
      value: protocolConfig?.treasury?.toBase58() || '',
      description: 'Account that receives protocol fees',
      type: 'admin',
    },
    {
      name: 'Authority Account',
      field: 'authority',
      value: protocolConfig?.authority?.toBase58() || '',
      description: 'Account authorized to update protocol parameters',
      type: 'admin',
    },
    {
      name: 'DEX TWAP Updater',
      field: 'dex_twap_updater',
      value: protocolConfig?.dexTwapUpdater?.toBase58() || '',
      description: 'Authorized account to update DEX TWAP price feeds',
      type: 'admin',
    },
  ];

  // Token & Market parameters
  const tokenParameters: Parameter[] = [
    {
      name: 'Token Expiration',
      field: 'token_expiration_seconds',
      value: protocolConfig?.tokenExpirationSeconds || 3600,
      unit: 'seconds',
      description: 'Time window to deploy liquidity after token mint before expiration',
      type: 'time',
      min: 300,
      max: 86400,
    },
  ];


  // Load protocol configuration and check authority
  useEffect(() => {
    async function loadProtocolConfig() {
      // Handle fallback mode (test data) - skip loading real protocol config
      if (fallback) {
        setProtocolConfig({
          authority: publicKey || new PublicKey('11111111111111111111111111111111'),
          treasury: publicKey || new PublicKey('11111111111111111111111111111111'),
          dexTwapUpdater: publicKey || new PublicKey('11111111111111111111111111111111'),
          mintFee: 1000000, // 0.001 SOL
          defaultProtocolFeeRate: 8,
          defaultCreatorFeeRate: 2,
          maxProtocolFeeRate: 50,
          depegThresholdBps: 50,
          depegRequiredObs: 3,
          clearRequiredObs: 5,
          dexTwapWindowSecs: 900,
          dexTwapStaleAgeSecs: 1800,
          mintPerSlotCapFeelssol: 0,
          redeemPerSlotCapFeelssol: 0,
          tokenExpirationSeconds: 3600,
        });
        setIsAuthority(true); // In test mode, assume authority
        setLoading(false);
        return;
      }

      if (!program || !publicKey) {
        setLoading(false);
        return;
      }

      // Derive protocol config PDA
      const [protocolConfigPDA] = PublicKey.findProgramAddressSync(
        [PROTOCOL_CONFIG_SEED],
        program.programId
      );

      try {
        setLoadError(null);

        // Check if program has the expected structure
        // Try different possible account names due to IDL generation variations
        // Check if we have the account type available
        let resolvedAccountName = 'protocolConfig';
        
        // In newer Anchor versions, accounts might not be directly on program.account
        // We'll need to use the raw fetch method instead
        if (!(program as any).account?.[resolvedAccountName]) {
          // Check other possible names
          const possibleNames = [
            'feels::state::protocol_config::ProtocolConfig',
            'ProtocolConfig',
            'protocol_config'
          ];
          
          let found = false;
          for (const name of possibleNames) {
            if ((program as any).account?.[name]) {
              resolvedAccountName = name;
              found = true;
              break;
            }
          }
          
          // If still not found, we'll use the raw provider
          if (!found) {
            // We'll fetch directly using the provider
            const accountData = await connection.getAccountInfo(protocolConfigPDA);
            if (!accountData) {
              // Protocol not initialized - set error state instead of throwing
              setLoadError('Protocol config account not found on chain. The protocol may not be initialized yet.');
              setProtocolConfig(null);
              setLoading(false);
              return;
            }
            
            // For now, we'll skip the deserialization and just check if account exists
            // The actual config will need proper deserialization based on the IDL
            setProtocolConfig({
              authority: publicKey, // Temporary - we can't decode without proper IDL
              paused: false,
              updater: publicKey,
              default_pool_params: {
                base_fee_bps: 300,
                dynamic_fee_floor_bps: 10,
                dynamic_fee_scale_factor: 10000,
                time_fee_bps_per_second: 1,
                surge_threshold_bps: 1000,
                leverage_fee_scale_factor: 10000,
              }
            } as any);
            setIsAuthority(true); // Assume authority for now
            setLoading(false);
            return;
          }
        }
        setAccountName(resolvedAccountName);

        // Fetch protocol config account
        // @ts-ignore - Dynamic Anchor types
        const config = await (program as any).account[resolvedAccountName].fetch(protocolConfigPDA);
        setProtocolConfig(config);

        // Check if current wallet is the authority
        setIsAuthority(config.authority.equals(publicKey));

        setLoading(false);
      } catch (error) {
        console.error('Failed to load protocol config:', error);
        let errorMessage = 'Failed to load protocol configuration';
        
        if (error instanceof Error) {
          errorMessage = error.message;
          // Check for specific error cases
          if (error.message.includes('Account does not exist') || 
              error.message.includes('AccountNotFound') ||
              error.message.includes('could not find account')) {
            errorMessage = 'Protocol not initialized. Account does not exist at address: ' + protocolConfigPDA.toBase58();
          } else if (error.message.includes('Invalid account discriminator')) {
            errorMessage = 'Invalid protocol account data. The account exists but has incorrect data.';
          }
        }
        
        setLoadError(errorMessage);
        setLoading(false);
      }
    }

    loadProtocolConfig();
  }, [program, publicKey, connection, fallback]);

  const handleParameterChange = (field: string, value: any) => {
    setPendingChanges(prev => ({
      ...prev,
      [field]: value
    }));
  };

  const hasChanges = () => Object.keys(pendingChanges).length > 0;

  const submitChanges = async () => {
    if (!program || !publicKey || !hasChanges()) return;

    setSubmitting(true);
    try {
      // Derive PDAs
      const [protocolConfigPDA] = PublicKey.findProgramAddressSync(
        [PROTOCOL_CONFIG_SEED],
        program.programId
      );

      // Build update parameters with optional fields
      const updateParams = {
        mintFee: pendingChanges['mint_fee'] ? new BN(pendingChanges['mint_fee'] * 1e9) : null,
        treasury: pendingChanges['treasury'] ? new PublicKey(pendingChanges['treasury']) : null,
        authority: pendingChanges['authority'] ? new PublicKey(pendingChanges['authority']) : null,
        defaultProtocolFeeRate: pendingChanges['default_protocol_fee_rate'] || null,
        defaultCreatorFeeRate: pendingChanges['default_creator_fee_rate'] || null,
        maxProtocolFeeRate: pendingChanges['max_protocol_fee_rate'] || null,
        tokenExpirationSeconds: pendingChanges['token_expiration_seconds'] || null,
        dexTwapUpdater: pendingChanges['dex_twap_updater'] ? new PublicKey(pendingChanges['dex_twap_updater']) : null,
        depegThresholdBps: pendingChanges['depeg_threshold_bps'] || null,
        depegRequiredObs: pendingChanges['depeg_required_obs'] || null,
        clearRequiredObs: pendingChanges['clear_required_obs'] || null,
        dexTwapWindowSecs: pendingChanges['dex_twap_window_secs'] || null,
        dexTwapStaleAgeSecs: pendingChanges['dex_twap_stale_age_secs'] || null,
        dexWhitelist: pendingChanges['dex_whitelist'] || null,
        mintPerSlotCapFeelssol: pendingChanges['mint_per_slot_cap_feelssol'] ? new BN(pendingChanges['mint_per_slot_cap_feelssol'] * 1e9) : null,
        redeemPerSlotCapFeelssol: pendingChanges['redeem_per_slot_cap_feelssol'] ? new BN(pendingChanges['redeem_per_slot_cap_feelssol'] * 1e9) : null,
      };

      // Submit update transaction
      // @ts-ignore - Dynamic Anchor types
      const tx = await program.methods['updateProtocol'](updateParams)
        .accounts({
          authority: publicKey,
          protocolConfig: protocolConfigPDA,
        })
        .rpc();

      toast({
        title: 'Protocol Updated',
        description: `Transaction: ${tx.slice(0, 8)}...${tx.slice(-8)}`,
      });

      // Reload config using the same account name resolution
      // @ts-ignore - Dynamic Anchor types
      const newConfig = await program.account[accountName].fetch(protocolConfigPDA);
      setProtocolConfig(newConfig);
      setPendingChanges({});
      setEditMode(false);

    } catch (error) {
      console.error('Failed to update protocol:', error);
      toast({
        title: 'Update Failed',
        description: error instanceof Error ? error.message : 'Unknown error',
        variant: 'destructive',
      });
    } finally {
      setSubmitting(false);
    }
  };

  const renderParameterInput = (param: Parameter) => {
    const currentValue = pendingChanges[param.field] ?? param.value;
    
    if (typeof param.value === 'boolean') {
      return (
        <Switch
          checked={currentValue as boolean}
          onCheckedChange={(checked) => handleParameterChange(param.field, checked)}
          disabled={!editMode}
        />
      );
    }

    // Handle Pubkey fields (string type for admin parameters)
    if (param.type === 'admin' && typeof param.value === 'string') {
      return (
        <Input
          type="text"
          value={currentValue as string}
          onChange={(e) => handleParameterChange(param.field, e.target.value)}
          disabled={!editMode}
          placeholder="Enter valid Solana public key..."
          className="w-80 font-mono text-xs"
        />
      );
    }

    return (
      <Input
        type="number"
        value={currentValue}
        onChange={(e) => {
          const value = param.decimals ? parseFloat(e.target.value) : parseInt(e.target.value);
          handleParameterChange(param.field, value);
        }}
        min={param.min}
        max={param.max}
        step={param.decimals ? Math.pow(10, -param.decimals) : 1}
        disabled={!editMode}
        className="w-24 text-right font-mono"
      />
    );
  };

  if (loading) {
    return (
      <Card className="w-full">
        <CardHeader>
          <CardTitle className="text-xl">Protocol Parameters</CardTitle>
          <CardDescription>Loading protocol configuration...</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin" />
          </div>
        </CardContent>
      </Card>
    );
  }

  if (!program && !loadError) {
    return (
      <Card className="w-full">
        <CardHeader>
          <CardTitle className="text-xl">Protocol Parameters</CardTitle>
          <CardDescription>Connect wallet to view parameters</CardDescription>
        </CardHeader>
      </Card>
    );
  }

  if (loadError) {
    return (
      <Card className="w-full">
        <CardHeader>
          <CardTitle className="text-xl">Protocol Parameters</CardTitle>
          <CardDescription className="text-destructive">Configuration Not Available</CardDescription>
        </CardHeader>
        <CardContent>
          <Alert className="border-destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription className="space-y-2">
              <p>The protocol configuration could not be loaded. This may be because the protocol hasn&apos;t been initialized yet.</p>
              {(loadError.includes('Account does not exist') || loadError.includes('not found')) && (
                <div className="mt-2">
                  <p className="text-sm font-medium">To initialize the protocol:</p>
                  <ol className="text-sm list-decimal list-inside mt-1 space-y-1">
                    <li>Ensure the Feels program is deployed</li>
                    <li>Run the protocol initialization instruction</li>
                    <li>This is typically done during the first deployment</li>
                  </ol>
                </div>
              )}
            </AlertDescription>
          </Alert>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="w-full">
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="text-xl">Protocol Parameters</CardTitle>
            <CardDescription>
              {fallback ? (
                <span className="text-amber-600">
                  Test data mode - showing placeholder protocol parameters
                </span>
              ) : (
                isAuthority ? 'Manage protocol configuration' : 'View current protocol settings'
              )}
            </CardDescription>
          </div>
          {isAuthority && (
            <div className="flex items-center gap-2">
              {editMode ? (
                <>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      setEditMode(false);
                      setPendingChanges({});
                    }}
                    disabled={submitting}
                  >
                    Cancel
                  </Button>
                  <Button
                    size="sm"
                    onClick={submitChanges}
                    disabled={!hasChanges() || submitting}
                  >
                    {submitting && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                    Save Changes
                  </Button>
                </>
              ) : (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setEditMode(true)}
                >
                  <Unlock className="h-4 w-4 mr-2" />
                  Edit Parameters
                </Button>
              )}
            </div>
          )}
        </div>
      </CardHeader>
      <CardContent>
        {isAuthority ? (
          <>
            <Alert className="mb-6">
              <Lock className="h-4 w-4" />
              <AlertDescription>
                You are the protocol authority. Changes will take effect immediately on-chain.
              </AlertDescription>
            </Alert>

            <Tabs defaultValue="fees" className="w-full">
              <TabsList className="grid w-full grid-cols-4">
                <TabsTrigger value="fees">Fees</TabsTrigger>
                <TabsTrigger value="safety">Safety</TabsTrigger>
                <TabsTrigger value="admin">Admin</TabsTrigger>
                <TabsTrigger value="token">Token</TabsTrigger>
              </TabsList>

              <TabsContent value="fees" className="space-y-4">
                {feeParameters.map((param) => (
                  <div key={param.field} className="space-y-2">
                    <div className="flex items-center justify-between">
                      <div className="flex-1">
                        <Label className="text-sm font-medium">{param.name}</Label>
                        {param.description && (
                          <p className="text-xs text-muted-foreground">{param.description}</p>
                        )}
                      </div>
                      <div className="flex items-center gap-2">
                        {renderParameterInput(param)}
                        {param.unit && (
                          <span className="text-sm text-muted-foreground">{param.unit}</span>
                        )}
                      </div>
                    </div>
                  </div>
                ))}
              </TabsContent>

              <TabsContent value="safety" className="space-y-4">
                {safetyParameters.map((param) => (
                  <div key={param.field} className="space-y-2">
                    <div className="flex items-center justify-between">
                      <div className="flex-1">
                        <Label className="text-sm font-medium">{param.name}</Label>
                        {param.description && (
                          <p className="text-xs text-muted-foreground">{param.description}</p>
                        )}
                      </div>
                      <div className="flex items-center gap-2">
                        {renderParameterInput(param)}
                        {param.unit && (
                          <span className="text-sm text-muted-foreground">{param.unit}</span>
                        )}
                      </div>
                    </div>
                  </div>
                ))}
              </TabsContent>

              <TabsContent value="admin" className="space-y-4">
                <Alert className="mb-4">
                  <Shield className="h-4 w-4" />
                  <AlertDescription>
                    Administrative accounts control protocol access and updates. Handle with extreme care.
                  </AlertDescription>
                </Alert>
                {adminParameters.map((param) => (
                  <div key={param.field} className="space-y-2">
                    <div className="flex flex-col gap-2">
                      <div>
                        <Label className="text-sm font-medium">{param.name}</Label>
                        {param.description && (
                          <p className="text-xs text-muted-foreground">{param.description}</p>
                        )}
                      </div>
                      <div className="flex items-center gap-2">
                        {renderParameterInput(param)}
                      </div>
                    </div>
                  </div>
                ))}
              </TabsContent>

              <TabsContent value="token" className="space-y-4">
                {tokenParameters.map((param) => (
                  <div key={param.field} className="space-y-2">
                    <div className="flex items-center justify-between">
                      <div className="flex-1">
                        <Label className="text-sm font-medium">{param.name}</Label>
                        {param.description && (
                          <p className="text-xs text-muted-foreground">{param.description}</p>
                        )}
                      </div>
                      <div className="flex items-center gap-2">
                        {renderParameterInput(param)}
                        {param.unit && (
                          <span className="text-sm text-muted-foreground">{param.unit}</span>
                        )}
                      </div>
                    </div>
                  </div>
                ))}
              </TabsContent>
            </Tabs>
          </>
        ) : (
          <>
            <Alert className="mb-6">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>
                View-only mode. Only the protocol authority can modify parameters.
              </AlertDescription>
            </Alert>

            {/* Display current parameters in read-only format */}
            <div className="space-y-6">
              <div>
                <h3 className="font-medium mb-3 flex items-center gap-2">
                  <Percent className="h-4 w-4" />
                  Fee Configuration
                </h3>
                <div className="space-y-2">
                  {feeParameters.map((param) => (
                    <div key={param.field} className="flex items-center justify-between p-2 rounded-lg hover:bg-muted/50">
                      <div>
                        <span className="text-sm font-medium">{param.name}</span>
                        {param.description && (
                          <p className="text-xs text-muted-foreground">{param.description}</p>
                        )}
                      </div>
                      <span className="text-sm font-mono">
                        {param.value} {param.unit}
                      </span>
                    </div>
                  ))}
                </div>
              </div>

              <div>
                <h3 className="font-medium mb-3 flex items-center gap-2">
                  <Shield className="h-4 w-4" />
                  Safety & Circuit Breaker
                </h3>
                <div className="space-y-2">
                  {safetyParameters.map((param) => (
                    <div key={param.field} className="flex items-center justify-between p-2 rounded-lg hover:bg-muted/50">
                      <div>
                        <span className="text-sm font-medium">{param.name}</span>
                        {param.description && (
                          <p className="text-xs text-muted-foreground">{param.description}</p>
                        )}
                      </div>
                      <span className="text-sm font-mono">
                        {typeof param.value === 'boolean' ? (
                          <Badge variant={param.value ? 'destructive' : 'default'}>
                            {param.value ? 'Paused' : 'Active'}
                          </Badge>
                        ) : (
                          `${param.value} ${param.unit || ''}`
                        )}
                      </span>
                    </div>
                  ))}
                </div>
              </div>

              <div>
                <h3 className="font-medium mb-3 flex items-center gap-2">
                  <Settings className="h-4 w-4" />
                  Administrative Accounts
                </h3>
                <div className="space-y-2">
                  {adminParameters.map((param) => (
                    <div key={param.field} className="p-2 rounded-lg hover:bg-muted/50">
                      <div className="flex flex-col gap-1">
                        <span className="text-sm font-medium">{param.name}</span>
                        {param.description && (
                          <p className="text-xs text-muted-foreground">{param.description}</p>
                        )}
                        <span className="text-xs font-mono text-muted-foreground break-all">
                          {param.value || 'Not set'}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              </div>

              <div>
                <h3 className="font-medium mb-3 flex items-center gap-2">
                  <Info className="h-4 w-4" />
                  Token & Market Parameters
                </h3>
                <div className="space-y-2">
                  {tokenParameters.map((param) => (
                    <div key={param.field} className="flex items-center justify-between p-2 rounded-lg hover:bg-muted/50">
                      <div>
                        <span className="text-sm font-medium">{param.name}</span>
                        {param.description && (
                          <p className="text-xs text-muted-foreground">{param.description}</p>
                        )}
                      </div>
                      <span className="text-sm font-mono">
                        {param.value} {param.unit}
                      </span>
                    </div>
                  ))}
                </div>
              </div>

              <div>
                <h3 className="font-medium mb-3 flex items-center gap-2">
                  <Shield className="h-4 w-4" />
                  DEX Whitelist
                </h3>
                <div className="p-2 rounded-lg bg-muted/30">
                  <div className="flex flex-col gap-2">
                    <div className="flex items-center justify-between">
                      <span className="text-sm font-medium">Whitelisted DEX Venues</span>
                      <Badge variant="outline">
                        {protocolConfig?.dexWhitelistLen || 0} / 8
                      </Badge>
                    </div>
                    <p className="text-xs text-muted-foreground">
                      Authorized DEX pools for TWAP price feeds
                    </p>
                    {protocolConfig?.dexWhitelist && protocolConfig.dexWhitelistLen > 0 ? (
                      <div className="space-y-1 mt-2">
                        {protocolConfig.dexWhitelist.slice(0, protocolConfig.dexWhitelistLen).map((pubkey: any, index: number) => (
                          <div key={index} className="text-xs font-mono bg-background px-2 py-1 rounded border">
                            {pubkey.toBase58()}
                          </div>
                        ))}
                      </div>
                    ) : (
                      <p className="text-xs text-muted-foreground italic">No DEX venues whitelisted</p>
                    )}
                  </div>
                </div>
              </div>
            </div>
          </>
        )}

        {/* Protocol Authority Info */}
        <div className="mt-6 p-4 bg-muted rounded-lg">
          <div className="flex items-start gap-2">
            <Info className="h-4 w-4 mt-0.5" />
            <div className="flex-1">
              <div className="flex items-center justify-between mb-2">
                <p className="text-sm font-medium">Protocol Authority</p>
                {publicKey && protocolConfig?.authority && (
                  <Badge variant={isAuthority ? "default" : "outline"} className="text-xs">
                    {isAuthority ? "You are authority" : "Read-only access"}
                  </Badge>
                )}
              </div>
              
              <div className="space-y-2">
                <div>
                  <p className="text-xs text-muted-foreground">Registered Authority Account:</p>
                  <p className="text-xs font-mono bg-background px-2 py-1 rounded border mt-1 break-all">
                    {protocolConfig?.authority ? 
                      protocolConfig.authority.toBase58()
                      : 'Not loaded'}
                  </p>
                </div>
                
                {publicKey && (
                  <div>
                    <p className="text-xs text-muted-foreground">Your Connected Account:</p>
                    <p className="text-xs font-mono bg-background px-2 py-1 rounded border mt-1 break-all">
                      {publicKey.toBase58()}
                    </p>
                  </div>
                )}
                
                <div className="pt-2 border-t border-border">
                  <p className="text-xs text-muted-foreground">
                    {isAuthority 
                      ? "You have full administrative access to modify all protocol parameters."
                      : publicKey 
                        ? "You can view all parameters but cannot modify them. Only the registered authority can make changes."
                        : "Connect your wallet to see your access level."
                    }
                  </p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}