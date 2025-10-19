import { NextRequest, NextResponse } from 'next/server';
import { uploadCache } from '@/services/upload-cache';

export async function POST(
  _request: NextRequest,
  { params }: { params: Promise<{ uploadId: string }> }
) {
  try {
    const { uploadId } = await params;

    if (!uploadId) {
      return NextResponse.json(
        { error: 'Upload ID is required' },
        { status: 400 }
      );
    }

    // Get upload record to verify it exists
    const record = uploadCache.getUploadRecord(uploadId);

    if (!record) {
      return NextResponse.json(
        { error: 'Upload record not found' },
        { status: 404 }
      );
    }

    // Confirm the upload to extend its TTL
    uploadCache.confirmUpload(uploadId);

    return NextResponse.json({
      message: 'Upload confirmed successfully',
      uploadId
    });

  } catch (error) {
    console.error('Confirm metadata error:', error);
    return NextResponse.json(
      { error: 'Failed to confirm metadata' },
      { status: 500 }
    );
  }
}