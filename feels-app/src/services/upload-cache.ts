import crypto from 'crypto';

interface CacheEntry {
  uri: string;
  uploadId: string;
  timestamp: number;
  metadata: any;
}

interface UploadRecord {
  uri: string;
  metadata: any;
  timestamp: number;
}

class UploadCache {
  private hashCache: Map<string, CacheEntry> = new Map();
  private uploadRecords: Map<string, UploadRecord> = new Map();
  private cacheTTL: number;

  constructor() {
    this.cacheTTL = parseInt(process.env['UPLOAD_CACHE_TTL'] || '1800000'); // 30 minutes default
    
    // Run cleanup every 5 minutes
    setInterval(() => this.cleanup(), 5 * 60 * 1000);
  }

  // Generate hash from image data
  generateHash(imageData: Buffer): string {
    return crypto.createHash('sha256').update(imageData).digest('hex');
  }

  // Check if image already exists
  checkHash(hash: string): CacheEntry | null {
    const entry = this.hashCache.get(hash);
    if (entry && Date.now() - entry.timestamp < this.cacheTTL) {
      return entry;
    }
    return null;
  }

  // Store new upload in cache
  storeUpload(hash: string, uri: string, metadata: any): string {
    const uploadId = crypto.randomUUID();
    const timestamp = Date.now();
    
    // Store in hash cache
    this.hashCache.set(hash, {
      uri,
      uploadId,
      timestamp,
      metadata
    });
    
    // Store upload record for recovery
    this.uploadRecords.set(uploadId, {
      uri,
      metadata,
      timestamp
    });
    
    return uploadId;
  }

  // Get upload record for retry scenarios
  getUploadRecord(uploadId: string): UploadRecord | null {
    const record = this.uploadRecords.get(uploadId);
    if (record && Date.now() - record.timestamp < this.cacheTTL) {
      return record;
    }
    return null;
  }

  // Confirm successful transaction, optionally extend TTL
  confirmUpload(uploadId: string): void {
    const record = this.uploadRecords.get(uploadId);
    if (record) {
      // Extend TTL by another 30 minutes for popular uploads
      record.timestamp = Date.now();
    }
  }

  // Clean up expired entries
  private cleanup(): void {
    const now = Date.now();
    
    // Clean hash cache
    for (const [hash, entry] of this.hashCache.entries()) {
      if (now - entry.timestamp > this.cacheTTL) {
        this.hashCache.delete(hash);
      }
    }
    
    // Clean upload records
    for (const [id, record] of this.uploadRecords.entries()) {
      if (now - record.timestamp > this.cacheTTL) {
        this.uploadRecords.delete(id);
      }
    }
  }

  // Get cache statistics
  getStats() {
    return {
      hashCacheSize: this.hashCache.size,
      uploadRecordsSize: this.uploadRecords.size,
      totalCacheEntries: this.hashCache.size + this.uploadRecords.size
    };
  }
}

// Export singleton instance
export const uploadCache = new UploadCache();