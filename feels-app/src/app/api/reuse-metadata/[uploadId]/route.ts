import { NextRequest, NextResponse } from 'next/server';
import { uploadCache } from '@/services/upload-cache';

export async function GET(
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

    // Get upload record from cache
    const record = uploadCache.getUploadRecord(uploadId);

    if (!record) {
      return NextResponse.json(
        { error: 'Upload record not found or expired' },
        { status: 404 }
      );
    }

    // Return the stored metadata
    return NextResponse.json({
      uri: record.uri,
      metadata: record.metadata,
      uploadId
    });

  } catch (error) {
    console.error('Reuse metadata error:', error);
    return NextResponse.json(
      { error: 'Failed to retrieve metadata' },
      { status: 500 }
    );
  }
}