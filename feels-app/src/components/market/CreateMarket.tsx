'use client';

import { useState } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { Connection } from '@solana/web3.js';
import { FEELS_IDL } from '@/sdk/sdk';
import { createFeelsProgram } from '@/sdk/program-workaround';
import { PROTOCOL_CONSTANTS, PDA_SEEDS } from '@/constants/constants';
import { getFeelsSOLMint, ensureLocalnetTokensLoaded } from '@/constants/localnet-tokens';
import { getMetaplexProgramId, ensureMetaplexConfigLoaded } from '@/constants/metaplex-config';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Loader2, AlertCircle, CheckCircle, Plus } from 'lucide-react';

interface CreateMarketProps {
  connection: Connection;
  onMarketCreated?: (marketAddress: string) => void;
}

interface MarketParams {
  // Token parameters (for mint_token)
  tokenName: string;
  tokenSymbol: string; // ticker
  tokenUri: string;
  
  // Market parameters (for initialize_market)
  baseFeesBps: number;
  tickSpacing: number;
  initialSqrtPrice: string; // Q64 format as string for precision
  initialBuyFeelsSOLAmount: number;
  
  // Liquidity deployment parameters (for deploy_initial_liquidity)
  tickStepSize: number;
}

export function CreateMarket({ connection, onMarketCreated }: CreateMarketProps) {
  const { publicKey, signTransaction, signAllTransactions, connected } = useWallet();
  const [isCreating, setIsCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [params, setParams] = useState<MarketParams>({
    // Token parameters
    tokenName: '',
    tokenSymbol: '',
    tokenUri: '',
    
    // Market parameters
    baseFeesBps: PROTOCOL_CONSTANTS.DEFAULT_BASE_FEE_BPS, // 0.3%
    tickSpacing: PROTOCOL_CONSTANTS.DEFAULT_TICK_SPACING, // Standard tick spacing for most tokens
    initialSqrtPrice: '79228162514264337593543950336', // 1:1 price (2^96)
    initialBuyFeelsSOLAmount: 0, // Optional initial buy
    
    // Liquidity deployment parameters
    tickStepSize: PROTOCOL_CONSTANTS.DEFAULT_TICK_STEP_SIZE, // Ticks between each liquidity step
  });

  const handleCreateMarket = async () => {
    if (!publicKey || !signTransaction || !signAllTransactions) {
      setError('Wallet not connected');
      return;
    }

    setIsCreating(true);
    setError(null);
    setSuccess(null);

    try {
      // Ensure localnet tokens and metaplex config are loaded before proceeding
      await Promise.all([
        ensureLocalnetTokensLoaded(),
        ensureMetaplexConfigLoaded()
      ]);
      
      // Dynamically import heavy dependencies only when needed
      const [
        { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY, Keypair, Transaction },
        { Program, AnchorProvider, BN },
        { TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync },
        { FEELS_IDL, FEELS_PROGRAM_ID }
      ] = await Promise.all([
        import('@solana/web3.js'),
        import('@coral-xyz/anchor'),
        import('@solana/spl-token'),
        import('@/sdk/sdk')
      ]);

      // Create provider and program
      const provider = new AnchorProvider(
        connection,
        { publicKey, signTransaction, signAllTransactions } as any,
        { commitment: 'confirmed' }
      );
      
      // @ts-ignore - Anchor types are complex with dynamic imports
      const program = createFeelsProgram(provider);

      // Generate keypair for the new token mint
      const tokenMint = Keypair.generate();
      
      // Get FeelsSOL mint - dynamically loaded for the current environment
      const feelssolMint = getFeelsSOLMint();
      
      // Order tokens correctly (token_0 < token_1)
      const token0 = feelssolMint.toBase58() < tokenMint.publicKey.toBase58() ? feelssolMint : tokenMint.publicKey;
      const token1 = feelssolMint.toBase58() < tokenMint.publicKey.toBase58() ? tokenMint.publicKey : feelssolMint;

      // Derive PDAs
      const [escrow] = PublicKey.findProgramAddressSync(
        [Buffer.from(PDA_SEEDS.ESCROW), tokenMint.publicKey.toBuffer()],
        program.programId
      );

      const [escrowAuthority] = PublicKey.findProgramAddressSync(
        [Buffer.from(PDA_SEEDS.ESCROW_AUTHORITY), escrow.toBuffer()],
        program.programId
      );

      const metaplexProgramId = getMetaplexProgramId();
      const [metadata] = PublicKey.findProgramAddressSync(
        [Buffer.from(PDA_SEEDS.METADATA), metaplexProgramId.toBuffer(), tokenMint.publicKey.toBuffer()],
        metaplexProgramId
      );

      const [market] = PublicKey.findProgramAddressSync(
        [Buffer.from(PDA_SEEDS.MARKET), token0.toBuffer(), token1.toBuffer()],
        program.programId
      );

      const [buffer] = PublicKey.findProgramAddressSync(
        [Buffer.from(PDA_SEEDS.BUFFER), market.toBuffer()],
        program.programId
      );

      const [oracle] = PublicKey.findProgramAddressSync(
        [Buffer.from(PDA_SEEDS.ORACLE), market.toBuffer()],
        program.programId
      );

      const [vault0] = PublicKey.findProgramAddressSync(
        [Buffer.from(PDA_SEEDS.VAULT), token0.toBuffer(), token1.toBuffer(), Buffer.from('0')],
        program.programId
      );

      const [vault1] = PublicKey.findProgramAddressSync(
        [Buffer.from(PDA_SEEDS.VAULT), token0.toBuffer(), token1.toBuffer(), Buffer.from('1')],
        program.programId
      );

      const [marketAuthority] = PublicKey.findProgramAddressSync(
        [Buffer.from(PDA_SEEDS.MARKET_AUTHORITY), market.toBuffer()],
        program.programId
      );

      const [protocolConfig] = PublicKey.findProgramAddressSync(
        [Buffer.from(PDA_SEEDS.PROTOCOL_CONFIG)],
        program.programId
      );

      const [protocolToken] = PublicKey.findProgramAddressSync(
        [Buffer.from(PDA_SEEDS.PROTOCOL_TOKEN), tokenMint.publicKey.toBuffer()],
        program.programId
      );

      const [tranchePlan] = PublicKey.findProgramAddressSync(
        [Buffer.from(PDA_SEEDS.TRANCHE_PLAN), market.toBuffer()],
        program.programId
      );

      const [treasury] = PublicKey.findProgramAddressSync(
        [Buffer.from(PDA_SEEDS.TREASURY)], // This might need adjustment based on actual treasury PDA
        program.programId
      );

      // Get associated token accounts
      const escrowTokenVault = getAssociatedTokenAddressSync(tokenMint.publicKey, escrowAuthority, true);
      const escrowFeelssolVault = getAssociatedTokenAddressSync(feelssolMint, escrowAuthority, true);
      const creatorFeelssolAccount = getAssociatedTokenAddressSync(feelssolMint, publicKey);
      const deployerFeelssolAccount = getAssociatedTokenAddressSync(feelssolMint, publicKey);
      const deployerTokenOut = getAssociatedTokenAddressSync(tokenMint.publicKey, publicKey);

      // Build combined transaction with all three instructions
      // @ts-ignore - Anchor types are complex with dynamic imports
      const mintTokenIx = await program.methods
        .mintToken({
          ticker: params.tokenSymbol,
          name: params.tokenName,
          uri: params.tokenUri,
        })
        .accounts({
          creator: publicKey,
          tokenMint: tokenMint.publicKey,
          escrow,
          escrowTokenVault,
          escrowFeelssolVault,
          escrowAuthority,
          metadata,
          feelssolMint,
          creatorFeelssol: creatorFeelssolAccount,
          protocolConfig,
          protocolToken,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
          metadataProgram: metaplexProgramId,
        })
        .instruction();

      // @ts-ignore - Anchor types are complex with dynamic imports
      const initializeMarketIx = await program.methods
        .initializeMarket({
          baseFeesBps: params.baseFeesBps,
          tickSpacing: params.tickSpacing,
          initialSqrtPrice: new BN(params.initialSqrtPrice),
          initialBuyFeelssolAmount: new BN(params.initialBuyFeelsSOLAmount * 1e9),
        })
        .accounts({
          creator: publicKey,
          token0,
          token1,
          market,
          buffer,
          oracle,
          vault0,
          vault1,
          marketAuthority,
          feelssolMint,
          protocolToken0: token0 === feelssolMint ? SystemProgram.programId : protocolToken, // dummy if FeelsSOL
          protocolToken1: token1 === feelssolMint ? SystemProgram.programId : protocolToken, // dummy if FeelsSOL
          escrow,
          creatorFeelssol: creatorFeelssolAccount,
          creatorTokenOut: deployerTokenOut,
          escrowAuthority,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .instruction();

      // @ts-ignore - Anchor types are complex with dynamic imports
      const deployLiquidityIx = await program.methods
        .deployInitialLiquidity({
          tickStepSize: params.tickStepSize,
          initialBuyFeelssolAmount: new BN(params.initialBuyFeelsSOLAmount * 1e9),
        })
        .accounts({
          deployer: publicKey,
          market,
          token0Mint: token0,
          token1Mint: token1,
          deployerFeelssol: deployerFeelssolAccount,
          deployerTokenOut,
          vault0,
          vault1,
          marketAuthority,
          buffer,
          oracle,
          escrow,
          escrowTokenVault,
          escrowFeelssolVault,
          escrowAuthority,
          protocolConfig,
          treasury,
          tranchePlan,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .instruction();

      // Create and send transaction
      const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
      const tx = new Transaction({
        feePayer: publicKey,
        blockhash,
        lastValidBlockHeight,
      });

      // Add all instructions in order
      tx.add(mintTokenIx);
      tx.add(initializeMarketIx);
      tx.add(deployLiquidityIx);

      // Sign with the token mint keypair first
      tx.partialSign(tokenMint);
      
      // Then sign with the wallet
      const signedTx = await signTransaction(tx);

      const signature = await connection.sendRawTransaction(signedTx.serialize());
      await connection.confirmTransaction(signature, 'confirmed');

      console.log('Complete market creation transaction:', signature);
      console.log('Market address:', market.toBase58());
      console.log('Token mint:', tokenMint.publicKey.toBase58());
      
      setSuccess(`Market created successfully! 
        Market: ${market.toBase58()}
        Token: ${tokenMint.publicKey.toBase58()}`);
      
      // Clear form
      setParams({
        tokenName: '',
        tokenSymbol: '',
        tokenUri: '',
        baseFeesBps: PROTOCOL_CONSTANTS.DEFAULT_BASE_FEE_BPS,
        tickSpacing: PROTOCOL_CONSTANTS.DEFAULT_TICK_SPACING,
        initialSqrtPrice: '79228162514264337593543950336',
        initialBuyFeelsSOLAmount: 0,
        tickStepSize: PROTOCOL_CONSTANTS.DEFAULT_TICK_STEP_SIZE,
      });

      // Callback
      if (onMarketCreated) {
        onMarketCreated(market.toBase58());
      }
    } catch (err) {
      console.error('Failed to create market:', err);
      setError(err instanceof Error ? err.message : 'Failed to create market');
    } finally {
      setIsCreating(false);
    }
  };

  const isFormValid = () => {
    return (
      params.tokenName.trim() !== '' &&
      params.tokenSymbol.trim() !== '' &&
      params.tokenUri.trim() !== '' &&
      params.tickSpacing > 0 &&
      params.baseFeesBps > 0 &&
      params.initialSqrtPrice.trim() !== '' &&
      params.tickStepSize > 0 &&
      params.initialBuyFeelsSOLAmount >= 0 && // Allow 0 for no initial buy
      connected
    );
  };

  return (
    <Card id="create-market-card">
      <CardHeader>
        <CardTitle id="create-market-title" className="flex items-center gap-2">
          <Plus className="h-5 w-5" />
          Create New Market
        </CardTitle>
        <CardDescription id="create-market-description">
          Deploy a new token and create a FeelsSOL trading market
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div id="create-market-form" className="space-y-4">
          {/* Token Information */}
          <div id="token-info-section" className="space-y-2">
            <h3 id="token-info-heading" className="text-sm font-medium">Token Information</h3>
            <div id="token-info-fields" className="grid grid-cols-2 gap-4">
              <div id="token-name-field">
                <Label htmlFor="tokenName">Token Name</Label>
                <Input
                  id="token-name-input"
                  value={params.tokenName}
                  onChange={(e) => setParams({ ...params, tokenName: e.target.value })}
                  placeholder="My Token"
                  disabled={isCreating}
                />
              </div>
              <div id="token-symbol-field">
                <Label htmlFor="tokenSymbol">Token Symbol</Label>
                <Input
                  id="token-symbol-input"
                  value={params.tokenSymbol}
                  onChange={(e) => setParams({ ...params, tokenSymbol: e.target.value.toUpperCase() })}
                  placeholder="MTK"
                  disabled={isCreating}
                  maxLength={10}
                />
              </div>
            </div>
            <div id="token-uri-field">
              <Label htmlFor="tokenUri">Metadata URI</Label>
              <Input
                id="token-uri-input"
                value={params.tokenUri}
                onChange={(e) => setParams({ ...params, tokenUri: e.target.value })}
                placeholder="https://example.com/token-metadata.json"
                disabled={isCreating}
              />
            </div>
          </div>

          {/* Market Parameters */}
          <div id="market-params-section" className="space-y-2">
            <h3 id="market-params-heading" className="text-sm font-medium">Market Parameters</h3>
            <div id="market-params-fields" className="grid grid-cols-2 gap-4">
              <div id="initial-price-field">
                <Label htmlFor="initialSqrtPrice">Initial Price (sqrt)</Label>
                <Input
                  id="initial-sqrt-price-input"
                  value={params.initialSqrtPrice}
                  onChange={(e) => setParams({ ...params, initialSqrtPrice: e.target.value })}
                  placeholder="79228162514264337593543950336"
                  disabled={isCreating}
                />
                <p className="text-xs text-muted-foreground mt-1">
                  Price as Q64 sqrt format (default is 1:1 ratio)
                </p>
              </div>
              <div id="initial-buy-field">
                <Label htmlFor="initialBuy">Initial Buy (FeelsSOL)</Label>
                <Input
                  id="initial-buy-input"
                  type="number"
                  value={params.initialBuyFeelsSOLAmount}
                  onChange={(e) => setParams({ ...params, initialBuyFeelsSOLAmount: parseFloat(e.target.value) || 0 })}
                  min="0"
                  step="0.1"
                  disabled={isCreating}
                />
                <p className="text-xs text-muted-foreground mt-1">
                  Optional initial purchase amount (0 = no initial buy)
                </p>
              </div>
              <div id="tick-spacing-field">
                <Label htmlFor="tickSpacing">Tick Spacing</Label>
                <Input
                  id="tick-spacing-input"
                  type="number"
                  value={params.tickSpacing}
                  onChange={(e) => setParams({ ...params, tickSpacing: parseInt(e.target.value) || PROTOCOL_CONSTANTS.DEFAULT_TICK_SPACING })}
                  min="1"
                  disabled={isCreating}
                />
                <p className="text-xs text-muted-foreground mt-1">
                  Lower = more granular prices
                </p>
              </div>
              <div id="base-fee-field">
                <Label htmlFor="baseFee">Base Fee (%)</Label>
                <Input
                  id="base-fee-input"
                  type="number"
                  value={params.baseFeesBps / 100}
                  onChange={(e) => setParams({ ...params, baseFeesBps: Math.round(parseFloat(e.target.value) * 100) || PROTOCOL_CONSTANTS.DEFAULT_BASE_FEE_BPS })}
                  min="0.01"
                  max="10"
                  step="0.01"
                  disabled={isCreating}
                />
                <p className="text-xs text-muted-foreground mt-1">
                  Trading fee percentage
                </p>
              </div>
              <div id="tick-step-size-field">
                <Label htmlFor="tickStepSize">Tick Step Size</Label>
                <Input
                  id="tick-step-size-input"
                  type="number"
                  value={params.tickStepSize}
                  onChange={(e) => setParams({ ...params, tickStepSize: parseInt(e.target.value) || PROTOCOL_CONSTANTS.DEFAULT_TICK_STEP_SIZE })}
                  min="1"
                  disabled={isCreating}
                />
                <p className="text-xs text-muted-foreground mt-1">
                  Ticks between each liquidity step
                </p>
              </div>
            </div>
          </div>

          {/* Error/Success Messages */}
          {error && (
            <Alert id="create-market-error-alert" variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription id="create-market-error-message">{error}</AlertDescription>
            </Alert>
          )}
          
          {success && (
            <Alert id="create-market-success-alert" className="bg-primary/10 border-primary/20">
              <CheckCircle className="h-4 w-4 text-primary" />
              <AlertDescription id="create-market-success-message" className="text-primary-foreground">{success}</AlertDescription>
            </Alert>
          )}

          {/* Submit Button */}
          <Button
            id="create-market-submit-button"
            onClick={handleCreateMarket}
            disabled={!isFormValid() || isCreating}
            className="w-full"
          >
            {isCreating ? (
              <>
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                Creating Market...
              </>
            ) : (
              <>
                <Plus className="h-4 w-4 mr-2" />
                Create Market
              </>
            )}
          </Button>

          {!connected && (
            <p id="wallet-not-connected-message" className="text-sm text-muted-foreground text-center">
              Please connect your wallet to create a market
            </p>
          )}
        </div>
      </CardContent>
    </Card>
  );
}