import { NextRequest, NextResponse } from 'next/server';
// import { PinataSDK } from 'pinata';
import sharp from 'sharp';
import { uploadCache } from '@/services/upload-cache';

// Rate limiting map (simple in-memory for now)
const rateLimitMap = new Map<string, { count: number; resetTime: number }>();

// Initialize Pinata (TODO: Fix API implementation)
// const pinata = new PinataSDK({ 
//   pinataJwt: process.env['PINATA_JWT']!,
// });

// Rate limiting check
function checkRateLimit(ip: string): boolean {
  const now = Date.now();
  const window = parseInt(process.env['RATE_LIMIT_WINDOW'] || '60000');
  const maxRequests = parseInt(process.env['RATE_LIMIT_MAX'] || '10');
  
  const userLimit = rateLimitMap.get(ip);
  
  if (!userLimit || now > userLimit.resetTime) {
    rateLimitMap.set(ip, { count: 1, resetTime: now + window });
    return true;
  }
  
  if (userLimit.count >= maxRequests) {
    return false;
  }
  
  userLimit.count++;
  return true;
}

export async function POST(request: NextRequest) {
  try {
    // Get client IP for rate limiting
    const ip = request.headers.get('x-forwarded-for') || 'unknown';
    
    // Check rate limit
    if (!checkRateLimit(ip)) {
      return NextResponse.json(
        { error: 'Rate limit exceeded. Please try again later.' },
        { status: 429 }
      );
    }

    // Parse form data
    const formData = await request.formData();
    const name = formData.get('name') as string;
    const symbol = formData.get('symbol') as string;
    const description = formData.get('description') as string || `A new token created on the Feels Protocol.`;
    const imageFile = formData.get('image') as File;

    // Validate inputs
    if (!name || !symbol || !imageFile) {
      return NextResponse.json(
        { error: 'Missing required fields: name, symbol, or image' },
        { status: 400 }
      );
    }

    // Convert File to Buffer
    const imageArrayBuffer = await imageFile.arrayBuffer();
    const imageBuffer = Buffer.from(imageArrayBuffer);

    // Validate image
    const metadata = await sharp(imageBuffer).metadata();
    
    if (!metadata.width || !metadata.height) {
      return NextResponse.json(
        { error: 'Invalid image file' },
        { status: 400 }
      );
    }

    // Check if image is square
    if (metadata.width !== metadata.height) {
      return NextResponse.json(
        { error: 'Image must be square (equal width and height)' },
        { status: 400 }
      );
    }

    // Process image with sharp
    const processedImageBuffer = await sharp(imageBuffer)
      .resize(512, 512) // Resize to standard size
      .png({ quality: 90 }) // Convert to PNG with good quality
      .toBuffer();

    // Check size limit (2MB)
    if (processedImageBuffer.length > 2 * 1024 * 1024) {
      return NextResponse.json(
        { error: 'Processed image exceeds 2MB limit' },
        { status: 400 }
      );
    }

    // Generate hash for caching
    const imageHash = uploadCache.generateHash(processedImageBuffer);
    
    // Check if this image already exists
    const cachedEntry = uploadCache.checkHash(imageHash);
    if (cachedEntry) {
      return NextResponse.json({
        uri: cachedEntry.uri,
        uploadId: cachedEntry.uploadId,
        cached: true
      });
    }

    // Convert image to base64 for embedding in metadata
    const base64Image = processedImageBuffer.toString('base64');
    const imageDataUri = `data:image/png;base64,${base64Image}`;

    // Create metadata object following Metaplex standard
    const tokenMetadata = {
      name,
      symbol,
      description,
      image: imageDataUri,
      attributes: [],
      properties: {
        files: [
          {
            uri: imageDataUri,
            type: 'image/png'
          }
        ],
        category: 'image'
      }
    };

    // Upload metadata to IPFS via Pinata
    // TODO: Update to new Pinata SDK API
    const result = { IpfsHash: 'placeholder-hash' }; // await pinata.upload.json(tokenMetadata);

    const ipfsUri = `ipfs://${result.IpfsHash}`;
    
    // Store in cache
    const uploadId = uploadCache.storeUpload(imageHash, ipfsUri, tokenMetadata);

    return NextResponse.json({
      uri: ipfsUri,
      uploadId,
      cached: false
    });

  } catch (error) {
    console.error('Upload metadata error:', error);
    
    // Check for specific Pinata errors
    if (error instanceof Error) {
      if (error.message.includes('Unauthorized')) {
        return NextResponse.json(
          { error: 'IPFS service configuration error' },
          { status: 500 }
        );
      }
      
      if (error.message.includes('UNSUPPORTED')) {
        return NextResponse.json(
          { error: 'Unsupported image format. Please use PNG or JPEG.' },
          { status: 400 }
        );
      }
    }
    
    return NextResponse.json(
      { error: 'Failed to upload metadata' },
      { status: 500 }
    );
  }
}

// Export runtime config for Vercel
export const runtime = 'nodejs';