import imageCompression from 'browser-image-compression';

export interface ProcessedImage {
  file: File;
  preview: string;
}

export const IMAGE_CONSTRAINTS = {
  maxSizeMB: 2,
  maxWidthOrHeight: 512,
  useWebWorker: true,
};

export async function processImage(file: File): Promise<ProcessedImage> {
  // Validate file type
  if (!file.type.startsWith('image/')) {
    throw new Error('File must be an image');
  }

  // Create object URL for preview
  const preview = URL.createObjectURL(file);

  try {
    // Check if image is square
    const img = new Image();
    await new Promise((resolve, reject) => {
      img.onload = resolve;
      img.onerror = reject;
      img.src = preview;
    });

    if (img.width !== img.height) {
      // Return the file as-is if not square, will need cropping
      return {
        file,
        preview,
      };
    }

    // Compress and resize image
    const compressedFile = await imageCompression(file, {
      ...IMAGE_CONSTRAINTS,
      fileType: 'image/png',
    });

    // Check final size
    if (compressedFile.size > IMAGE_CONSTRAINTS.maxSizeMB * 1024 * 1024) {
      throw new Error(`Image must be less than ${IMAGE_CONSTRAINTS.maxSizeMB}MB after compression`);
    }

    return {
      file: new File([compressedFile], file.name, { type: 'image/png' }),
      preview,
    };
  } catch (error) {
    // Clean up preview URL on error
    URL.revokeObjectURL(preview);
    throw error;
  }
}

export async function processCroppedImage(blob: Blob, originalName: string): Promise<ProcessedImage> {
  // Create file from blob
  const file = new File([blob], originalName, { type: 'image/png' });
  
  // Create object URL for preview
  const preview = URL.createObjectURL(file);

  try {
    // Compress and resize image
    const compressedFile = await imageCompression(file, {
      ...IMAGE_CONSTRAINTS,
      fileType: 'image/png',
    });

    // Check final size
    if (compressedFile.size > IMAGE_CONSTRAINTS.maxSizeMB * 1024 * 1024) {
      throw new Error(`Image must be less than ${IMAGE_CONSTRAINTS.maxSizeMB}MB after compression`);
    }

    return {
      file: new File([compressedFile], originalName, { type: 'image/png' }),
      preview,
    };
  } catch (error) {
    // Clean up preview URL on error
    URL.revokeObjectURL(preview);
    throw error;
  }
}

export function cleanupPreview(preview: string) {
  URL.revokeObjectURL(preview);
}

export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' bytes';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}