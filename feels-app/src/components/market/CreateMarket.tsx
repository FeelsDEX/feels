'use client';

import { useState, useCallback } from 'react';
import { useWallet } from '@solana/wallet-adapter-react';
import { Connection } from '@solana/web3.js';
import { createFeelsProgram } from '@/program/program-workaround';
import { PROTOCOL_CONSTANTS, PDA_SEEDS } from '@/constants/protocol';
import { getFeelsSOLMint, ensureLocalnetTokensLoaded, getMetaplexProgramId, ensureMetaplexConfigLoaded } from '@/constants/localnet';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Loader2, AlertCircle, CheckCircle, Plus, ImageUp, X, Sparkles } from 'lucide-react';
import { useDropzone } from 'react-dropzone';
import { processImage, processCroppedImage, cleanupPreview, formatFileSize, ProcessedImage } from '@/utils/image-processing';
import { ImageCropper } from '@/components/content/ImageCropper';
import { useVanityAddress } from '@/contexts/VanityAddressContext';
import { useDataSource } from '@/contexts/DataSourceContext';

interface CreateMarketProps {
  connection: Connection;
  onMarketCreated?: (marketAddress: string) => void;
}

interface MarketParams {
  // Token parameters (for mint_token)
  tokenName: string;
  tokenSymbol: string; // ticker
  tokenImage?: ProcessedImage; // Processed image for upload
  uploadId?: string; // Upload ID for recovery
  
  // Optional metadata parameters
  description?: string;
  websiteUrl?: string;
  xHandle?: string;
  telegramHandle?: string;
  
  // User-configurable parameters
  initialBuyFeelsSOLAmount: number;
  initialBuyFeelsSOLAmountString?: string; // Keep string representation for proper display
}

export function CreateMarket({ connection, onMarketCreated }: CreateMarketProps) {
  const { publicKey, signTransaction, signAllTransactions, connected } = useWallet();
  const vanityAddress = useVanityAddress();
  const { dataSource } = useDataSource();
  const isTestDataMode = dataSource === 'test';
  const [isCreating, setIsCreating] = useState(false);
  const [isUploading, setIsUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [cropperImage, setCropperImage] = useState<{ file: File; preview: string } | null>(null);
  const [params, setParams] = useState<MarketParams>({
    // Token parameters
    tokenName: '',
    tokenSymbol: '',
    
    // User-configurable parameters
    initialBuyFeelsSOLAmount: 0, // Optional initial buy
  });

  // Handle image drop
  const onDrop = useCallback(async (acceptedFiles: File[]) => {
    if (acceptedFiles.length === 0) return;
    
    const file = acceptedFiles[0];
    if (!file) return; // Additional safety check
    
    setError(null);
    
    try {
      const processed = await processImage(file);
      
      // Check if image needs cropping
      const img = new Image();
      img.src = processed.preview;
      await new Promise((resolve) => {
        img.onload = resolve;
      });
      
      if (img.width !== img.height) {
        // Image is not square, open cropper
        setCropperImage({ file: file, preview: processed.preview });
      } else {
        // Image is already square, use as-is
        setParams(prev => ({ 
          ...prev, 
          tokenImage: processed,
          uploadId: undefined // Clear any previous upload ID
        }));
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to process image');
    }
  }, []);

  const { getRootProps, getInputProps, isDragActive } = useDropzone({
    onDrop,
    accept: {
      'image/*': ['.png', '.jpg', '.jpeg', '.gif', '.webp']
    },
    maxFiles: 1,
    multiple: false
  });

  // Remove image
  const removeImage = useCallback(() => {
    if (params.tokenImage) {
      cleanupPreview(params.tokenImage.preview);
      setParams(prev => ({ 
        ...prev, 
        tokenImage: undefined,
        uploadId: undefined
      }));
    }
  }, [params.tokenImage]);

  // Handle crop complete
  const handleCropComplete = useCallback(async (croppedBlob: Blob) => {
    if (!cropperImage) return;
    
    try {
      const processed = await processCroppedImage(croppedBlob, cropperImage.file.name);
      setParams(prev => ({ 
        ...prev, 
        tokenImage: processed,
        uploadId: undefined
      }));
      
      // Clean up original image preview
      cleanupPreview(cropperImage.preview);
      setCropperImage(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to process cropped image');
    }
  }, [cropperImage]);

  // Handle crop cancel
  const handleCropCancel = useCallback(() => {
    if (cropperImage) {
      cleanupPreview(cropperImage.preview);
      setCropperImage(null);
    }
  }, [cropperImage]);

  // Upload metadata to IPFS
  const uploadMetadata = async (): Promise<string> => {
    if (!params.tokenImage && !params.uploadId) {
      throw new Error('No image selected');
    }

    // If we have an uploadId, try to reuse it
    if (params.uploadId) {
      try {
        const response = await fetch(`/api/reuse-metadata/${params.uploadId}`);
        if (response.ok) {
          const data = await response.json();
          return data.uri;
        }
      } catch (err) {
        console.warn('Failed to reuse metadata, will re-upload:', err);
      }
    }

    // Upload new metadata
    if (!params.tokenImage) {
      throw new Error('No image to upload');
    }

    const formData = new FormData();
    formData.append('name', params.tokenName);
    formData.append('symbol', params.tokenSymbol);
    
    // Use custom description if provided, otherwise use default
    const description = params.description || `The ${params.tokenName} token on Feels Protocol`;
    formData.append('description', description);
    
    formData.append('image', params.tokenImage.file);
    
    // Add optional metadata fields if provided
    if (params.websiteUrl) {
      formData.append('external_url', params.websiteUrl);
    }
    
    // Add social media attributes
    const attributes = [];
    if (params.xHandle) {
      attributes.push({
        trait_type: 'X Handle',
        value: `@${params.xHandle}`
      });
    }
    if (params.telegramHandle) {
      attributes.push({
        trait_type: 'Telegram',
        value: `@${params.telegramHandle}`
      });
    }
    
    if (attributes.length > 0) {
      formData.append('attributes', JSON.stringify(attributes));
    }

    const response = await fetch('/api/upload-metadata', {
      method: 'POST',
      body: formData
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || 'Failed to upload metadata');
    }

    const data = await response.json();
    
    // Store uploadId for recovery
    setParams(prev => ({ ...prev, uploadId: data.uploadId }));
    
    return data.uri;
  };

  // Validation functions for metadata fields
  const validateUrl = (url: string): boolean => {
    if (!url) return true; // Optional field
    try {
      new URL(url);
      return true;
    } catch {
      return false;
    }
  };

  const validateXHandle = (handle: string): boolean => {
    if (!handle) return true; // Optional field
    // X handles: 1-15 characters, alphanumeric and underscore, no spaces
    return /^[a-zA-Z0-9_]{1,15}$/.test(handle);
  };

  const validateTelegramHandle = (handle: string): boolean => {
    if (!handle) return true; // Optional field
    // Telegram handles: 5-32 characters, alphanumeric and underscore, must start with letter
    return /^[a-zA-Z][a-zA-Z0-9_]{4,31}$/.test(handle);
  };

  const validateDescription = (description: string): boolean => {
    if (!description) return true; // Optional field
    // Description: max 500 characters
    return description.length <= 500;
  };

  const validateMetadata = (): string | null => {
    if (params.websiteUrl && !validateUrl(params.websiteUrl)) {
      return 'Please enter a valid website URL (e.g., https://example.com)';
    }
    if (params.xHandle && !validateXHandle(params.xHandle)) {
      return 'X handle must be 1-15 characters, alphanumeric and underscore only';
    }
    if (params.telegramHandle && !validateTelegramHandle(params.telegramHandle)) {
      return 'Telegram handle must be 5-32 characters, start with a letter, alphanumeric and underscore only';
    }
    if (params.description && !validateDescription(params.description)) {
      return 'Description must be 500 characters or less';
    }
    return null;
  };

  const handleCreateMarket = async () => {
    if (!publicKey || !signTransaction || !signAllTransactions) {
      setError('Wallet not connected');
      return;
    }

    // Validate vanity address
    const vanityKeypair = vanityAddress.getSolanaKeypair();
    if (!vanityKeypair) {
      // In test data mode, the vanity address should be available immediately
      if (isTestDataMode) {
        setError('Development keypair not available. Please refresh the page.');
      } else {
        setError('Vanity address not ready');
      }
      return;
    }

    // Validate metadata fields
    const validationError = validateMetadata();
    if (validationError) {
      setError(validationError);
      return;
    }

    setIsCreating(true);
    setError(null);
    setSuccess(null);

    let metadataUri = '';
    
    try {
      // Upload metadata (image is required)
      setIsUploading(true);
      try {
        metadataUri = await uploadMetadata();
      } finally {
        setIsUploading(false);
      }
      // Ensure localnet tokens and metaplex config are loaded before proceeding
      await Promise.all([
        ensureLocalnetTokensLoaded(),
        ensureMetaplexConfigLoaded()
      ]);
      
      // Dynamically import heavy dependencies only when needed
      const [
        { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY, Transaction },
        { AnchorProvider, BN },
        { TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, Token }
      ] = await Promise.all([
        import('@solana/web3.js'),
        import('@coral-xyz/anchor'),
        import('@solana/spl-token')
      ]);

      // Create provider and program
      const provider = new AnchorProvider(
        connection,
        { publicKey, signTransaction, signAllTransactions } as any,
        { commitment: 'confirmed' }
      );
      
      // @ts-ignore - Anchor types are complex with dynamic imports
      const program = createFeelsProgram(provider);

      // Use the vanity keypair for the new token mint
      const tokenMint = vanityKeypair;
      
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
      const escrowTokenVault = await Token.getAssociatedTokenAddress(ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID, tokenMint.publicKey, escrowAuthority, true);
      const escrowFeelssolVault = await Token.getAssociatedTokenAddress(ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID, feelssolMint, escrowAuthority, true);
      const creatorFeelssolAccount = await Token.getAssociatedTokenAddress(ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID, feelssolMint, publicKey);
      const deployerFeelssolAccount = await Token.getAssociatedTokenAddress(ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID, feelssolMint, publicKey);
      const deployerTokenOut = await Token.getAssociatedTokenAddress(ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID, tokenMint.publicKey, publicKey);

      // Build combined transaction with all three instructions
      // @ts-ignore - Anchor types are complex with dynamic imports
      const mintTokenIx = await program.methods['mintToken']({
          ticker: params.tokenSymbol,
          name: params.tokenName,
          uri: metadataUri,
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
      const initializeMarketIx = await program.methods['initializeMarket']({
          baseFeesBps: PROTOCOL_CONSTANTS.DEFAULT_BASE_FEE_BPS,
          tickSpacing: PROTOCOL_CONSTANTS.DEFAULT_TICK_SPACING,
          initialSqrtPrice: new BN('5825507814218144'), // ~1e-7 FeelsSOL per token (tick -161216)
          initialBuyFeelssolAmount: new BN(params.initialBuyFeelsSOLAmount * 1e9),
        })
        .accounts({
          creator: publicKey,
          protocolConfig,
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
      const deployLiquidityIx = await program.methods['deployInitialLiquidity']({
          tickStepSize: PROTOCOL_CONSTANTS.DEFAULT_TICK_STEP_SIZE,
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
      
      // Confirm upload if using IPFS
      if (params.uploadId) {
        try {
          await fetch(`/api/confirm-metadata/${params.uploadId}`, { method: 'POST' });
        } catch (err) {
          console.warn('Failed to confirm metadata upload:', err);
        }
      }
      
      // Cleanup image preview
      if (params.tokenImage) {
        cleanupPreview(params.tokenImage.preview);
      }
      
      // Clear form
      setParams({
        tokenName: '',
        tokenSymbol: '',
        tokenImage: undefined,
        uploadId: undefined,
        description: undefined,
        websiteUrl: undefined,
        xHandle: undefined,
        telegramHandle: undefined,
        initialBuyFeelsSOLAmount: 0,
        initialBuyFeelsSOLAmountString: undefined,
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
    // In test data mode, vanity address should be available immediately
    const vanityAddressReady = isTestDataMode 
      ? vanityAddress.status.isReady 
      : vanityAddress.status.keypair !== null;
    
    return (
      params.tokenName.trim() !== '' &&
      params.tokenSymbol.trim() !== '' &&
      params.tokenImage !== undefined && // Image required
      params.initialBuyFeelsSOLAmount >= 0 && // Allow 0 for no initial buy
      vanityAddressReady && // Vanity address must be ready
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
            <h3 id="token-info-heading" className="text-lg font-semibold">Token Information</h3>
            <div id="token-info-fields" className="grid grid-cols-2 gap-4">
              <div id="token-name-field">
                <Label htmlFor="tokenName">Token Name *</Label>
                <Input
                  id="token-name-input"
                  value={params.tokenName}
                  onChange={(e) => setParams({ ...params, tokenName: e.target.value })}
                  placeholder="My Token"
                  disabled={isCreating}
                  className="focus:placeholder-transparent"
                  autoComplete="off"
                />
              </div>
              <div id="token-symbol-field">
                <Label htmlFor="tokenSymbol">Token Symbol *</Label>
                <Input
                  id="token-symbol-input"
                  value={params.tokenSymbol}
                  onChange={(e) => setParams({ ...params, tokenSymbol: e.target.value.toUpperCase() })}
                  placeholder="MTK"
                  disabled={isCreating}
                  className="focus:placeholder-transparent"
                  maxLength={10}
                  autoComplete="off"
                />
              </div>
            </div>
            <div id="token-image-field" className="col-span-2">
              <Label>Token Image *</Label>
              {!params.tokenImage ? (
                <div
                  {...getRootProps()}
                  className={`
                    border border-dashed rounded-lg p-6 text-center cursor-pointer
                    transition-colors duration-200
                    ${isDragActive ? 'border-primary bg-primary/10' : 'border-muted-foreground/40'}
                    ${isCreating || isUploading ? 'opacity-50 cursor-not-allowed' : ''}
                  `}
                  onMouseEnter={(e) => {
                    if (!isDragActive && !isCreating && !isUploading) {
                      e.currentTarget.style.backgroundColor = '#fafbfc';
                    }
                  }}
                  onMouseLeave={(e) => {
                    if (!isDragActive) {
                      e.currentTarget.style.backgroundColor = 'transparent';
                    }
                  }}
                >
                  <input {...getInputProps()} disabled={isCreating || isUploading} />
                  <ImageUp className="h-8 w-8 mx-auto mb-2 text-muted-foreground" strokeWidth={1.5} />
                  <p className="text-sm text-muted-foreground/60">
                    {isDragActive
                      ? 'Drop the image here'
                      : 'Drag & drop, or click to select image'}
                  </p>
                  <p className="text-xs text-muted-foreground/60 mt-1">
                  2MB max • Crop to square (PNG, JPG, GIF, WebP) 
                  </p>
                </div>
              ) : (
                <div className="relative border rounded-lg p-4" style={{ backgroundColor: '#fafbfc' }}>
                  <div className="flex items-center gap-4">
                    <img
                      src={params.tokenImage.preview}
                      alt="Token preview"
                      className="h-24 w-24 rounded-lg object-cover"
                    />
                    <div className="flex-1">
                      <p className="text-sm font-medium">{params.tokenImage.file.name}</p>
                      <p className="text-xs text-muted-foreground">
                        {formatFileSize(params.tokenImage.file.size)}
                      </p>
                      {params.uploadId && !isUploading && (
                        <p className="text-xs mt-1 text-success-500">Uploaded</p>
                      )}
                      {params.uploadId && isUploading && (
                        <p className="text-xs text-primary mt-1">Uploading...</p>
                      )}
                    </div>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={removeImage}
                      disabled={isCreating || isUploading}
                      className="h-8 w-8 p-0 hover:bg-gray-400"
                    >
                      <X className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              )}
            </div>
          </div>

          {/* Vanity Address Section */}
          <div id="vanity-address-section" className="space-y-2">
            <h3 id="vanity-address-heading" className="text-lg font-semibold">Token Address</h3>
            <div className="p-4 bg-muted rounded-lg space-y-3">
              {vanityAddress.status.keypair ? (
                <>
                  <div>
                    <p className="text-sm text-muted-foreground">Your token will be created at:</p>
                    <p className="font-mono text-sm break-all">{vanityAddress.status.keypair.publicKey}</p>
                  </div>
                  <div className="flex items-center gap-2 text-xs text-muted-foreground">
                    <Sparkles className="h-3 w-3" />
                    <span>Vanity address ready • Mined in {(vanityAddress.status.elapsedMs / 1000).toFixed(1)}s ({vanityAddress.status.attempts.toLocaleString()} attempts)</span>
                  </div>
                </>
              ) : (
                <div>
                  <p className="text-sm text-muted-foreground">Mining Feels address...</p>
                  <div className="flex items-center gap-2 mt-2">
                    <Loader2 className="h-4 w-4 animate-spin" />
                    <span className="font-mono text-xs">
                      {vanityAddress.status.attempts.toLocaleString()} attempts 
                      ({(vanityAddress.status.elapsedMs / 1000).toFixed(0)}s)
                    </span>
                  </div>
                </div>
              )}
            </div>
          </div>

          {/* Optional Metadata */}
          <div id="metadata-section" className="space-y-2">
            <h3 id="metadata-heading" className="text-lg font-semibold">Metadata (Optional)</h3>
            <div id="metadata-fields" className="space-y-4">
              <div id="description-field">
                <Label htmlFor="description">Description</Label>
                <textarea
                  id="description-input"
                  className="feels-input flex w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 resize-none transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium focus:placeholder-transparent"
                  style={{ height: '135px', minHeight: '135px' }}
                  value={params.description || ''}
                  onChange={(e) => setParams({ ...params, description: e.target.value })}
                  placeholder="Describe your token (max 500 characters)"
                  disabled={isCreating}
                  maxLength={500}
                  autoComplete="off"
                />
                <p className="text-xs text-muted-foreground mt-1">
                  {params.description ? `${params.description.length}/500 characters` : '0/500 characters'}
                </p>
              </div>
              
              <div id="social-fields" className="space-y-4">
                {/* Website URL on its own row */}
                <div id="website-field">
                  <Label htmlFor="websiteUrl">Website URL</Label>
                  <Input
                    id="website-input"
                    value={params.websiteUrl || ''}
                    onChange={(e) => setParams({ ...params, websiteUrl: e.target.value })}
                    placeholder="https://example.com"
                    disabled={isCreating}
                    className={`focus:placeholder-transparent ${params.websiteUrl && !validateUrl(params.websiteUrl) ? 'border-destructive' : ''}`}
                    autoComplete="off"
                  />
                  {params.websiteUrl && !validateUrl(params.websiteUrl) && (
                    <p className="text-xs text-destructive mt-1">Please enter a valid URL</p>
                  )}
                </div>
                
                {/* Social handles on the same row */}
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div id="x-handle-field">
                    <Label htmlFor="xHandle">X Handle</Label>
                    <div className="relative">
                      <span className="absolute left-3 top-1/2 transform -translate-y-1/2 text-muted-foreground text-sm">@</span>
                      <Input
                        id="x-handle-input"
                        value={params.xHandle || ''}
                        onChange={(e) => setParams({ ...params, xHandle: e.target.value })}
                        placeholder="username"
                        disabled={isCreating}
                        className={`focus:placeholder-transparent ${params.xHandle && !validateXHandle(params.xHandle) ? 'border-destructive' : ''}`}
                        style={{ paddingLeft: '1.65rem' }}
                        maxLength={15}
                        autoComplete="off"
                      />
                    </div>
                    {params.xHandle && !validateXHandle(params.xHandle) && (
                      <p className="text-xs text-destructive mt-1">1-15 characters, alphanumeric and underscore only</p>
                    )}
                  </div>
                  
                  <div id="telegram-handle-field">
                    <Label htmlFor="telegramHandle">Telegram Handle</Label>
                    <div className="relative">
                      <span className="absolute left-3 top-1/2 transform -translate-y-1/2 text-muted-foreground text-sm">@</span>
                      <Input
                        id="telegram-handle-input"
                        value={params.telegramHandle || ''}
                        onChange={(e) => setParams({ ...params, telegramHandle: e.target.value })}
                        placeholder="username"
                        disabled={isCreating}
                        className={`focus:placeholder-transparent ${params.telegramHandle && !validateTelegramHandle(params.telegramHandle) ? 'border-destructive' : ''}`}
                        style={{ paddingLeft: '1.65rem' }}
                        maxLength={32}
                        autoComplete="off"
                      />
                    </div>
                    {params.telegramHandle && !validateTelegramHandle(params.telegramHandle) && (
                      <p className="text-xs text-destructive mt-1">5-32 characters, start with letter, alphanumeric and underscore only</p>
                    )}
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* Market Parameters */}
          <div id="market-params-section" className="space-y-2">
            <h3 id="market-params-heading" className="text-lg font-semibold">Market Parameters</h3>
            <div id="initial-buy-field">
              <Label htmlFor="initialBuy">Initial Purchase Amount (FeelsSOL)</Label>
              <Input
                id="initial-buy-input"
                type="text"
                value={params.initialBuyFeelsSOLAmountString ?? (params.initialBuyFeelsSOLAmount === 0 ? '' : params.initialBuyFeelsSOLAmount.toString())}
                onChange={(e) => {
                  const value = e.target.value;
                  // Allow empty string, numbers with decimals
                  if (value === '' || /^\d*\.?\d*$/.test(value)) {
                    const numValue = value === '' ? 0 : parseFloat(value);
                    if (!isNaN(numValue) && numValue >= 0) {
                      setParams({ 
                        ...params, 
                        initialBuyFeelsSOLAmount: numValue,
                        initialBuyFeelsSOLAmountString: value // Keep the string representation
                      });
                    }
                  }
                }}
                onBlur={() => {
                  // Clear the string representation on blur to show the parsed number
                  setParams({ ...params, initialBuyFeelsSOLAmountString: undefined });
                }}
                placeholder="0"
                disabled={isCreating}
                className="focus:placeholder-transparent"
                autoComplete="off"
              />
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
            disabled={!isFormValid() || isCreating || isUploading}
            className="w-full"
          >
            {isUploading ? (
              <>
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                Uploading Metadata...
              </>
            ) : isCreating ? (
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
      
      {/* Image Cropper Dialog */}
      {cropperImage && (
        <ImageCropper
          image={cropperImage.preview}
          onCropComplete={handleCropComplete}
          onCancel={handleCropCancel}
          isOpen={!!cropperImage}
        />
      )}
    </Card>
  );
}