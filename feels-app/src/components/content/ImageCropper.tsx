'use client';

import { useState, useCallback } from 'react';
import Cropper from 'react-easy-crop';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Slider } from '@/components/ui/slider';
import { Label } from '@/components/ui/label';
import { Loader2, Crosshair } from 'lucide-react';

interface ImageCropperProps {
  image: string;
  onCropComplete: (croppedImage: Blob) => void;
  onCancel: () => void;
  isOpen: boolean;
}

interface CroppedArea {
  x: number;
  y: number;
  width: number;
  height: number;
}

export function ImageCropper({ image, onCropComplete, onCancel, isOpen }: ImageCropperProps) {
  const [crop, setCrop] = useState({ x: 0, y: 0 });
  const [zoom, setZoom] = useState(1);
  const [minZoom, setMinZoom] = useState(1);
  const [croppedAreaPixels, setCroppedAreaPixels] = useState<CroppedArea | null>(null);
  const [isProcessing, setIsProcessing] = useState(false);
  const [isLightImage, setIsLightImage] = useState(true); // Default to true to show dark grid initially

  const onCropChange = useCallback((crop: { x: number; y: number }) => {
    setCrop(crop);
  }, []);

  const onZoomChange = useCallback((zoom: number) => {
    setZoom(zoom);
  }, []);

  const onCropAreaChange = useCallback((_croppedArea: CroppedArea, croppedAreaPixels: CroppedArea) => {
    setCroppedAreaPixels(croppedAreaPixels);
  }, []);

  const createImage = (url: string): Promise<HTMLImageElement> => {
    return new Promise((resolve, reject) => {
      const image = new Image();
      image.addEventListener('load', () => resolve(image));
      image.addEventListener('error', (error) => reject(error));
      image.src = url;
    });
  };

  const getCroppedImg = async (
    imageSrc: string,
    pixelCrop: CroppedArea
  ): Promise<Blob> => {
    const image = await createImage(imageSrc);
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');

    if (!ctx) {
      throw new Error('No 2d context');
    }

    // Set canvas size to match the cropped area
    canvas.width = pixelCrop.width;
    canvas.height = pixelCrop.height;

    // Draw the cropped image
    ctx.drawImage(
      image,
      pixelCrop.x,
      pixelCrop.y,
      pixelCrop.width,
      pixelCrop.height,
      0,
      0,
      pixelCrop.width,
      pixelCrop.height
    );

    // Convert canvas to blob
    return new Promise((resolve, reject) => {
      canvas.toBlob(
        (blob) => {
          if (!blob) {
            reject(new Error('Canvas is empty'));
            return;
          }
          resolve(blob);
        },
        'image/png',
        0.9
      );
    });
  };

  const handleCrop = async () => {
    if (!croppedAreaPixels || isProcessing) return;

    // Delay setting processing state to avoid immediate visual change
    const processingTimeout = setTimeout(() => setIsProcessing(true), 100);
    
    try {
      const croppedImage = await getCroppedImg(image, croppedAreaPixels);
      clearTimeout(processingTimeout);
      onCropComplete(croppedImage);
    } catch (error) {
      clearTimeout(processingTimeout);
      console.error('Error cropping image:', error);
    } finally {
      setIsProcessing(false);
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent className="sm:max-w-[600px]">
        <DialogHeader>
          <DialogTitle>Crop Image</DialogTitle>
          <DialogDescription>
            Adjust your image to fit a square format. Zoom and reposition as needed.
          </DialogDescription>
        </DialogHeader>
        
        <div 
          key={isLightImage ? 'light' : 'dark'} 
          className={`relative h-[400px] bg-muted rounded-lg overflow-hidden ${isLightImage ? 'use-dark-grid' : 'use-light-grid'}`}
          style={{
            '--grid-color': isLightImage ? 'rgba(0, 0, 0, 0.4)' : 'rgba(255, 255, 255, 0.4)'
          } as React.CSSProperties}
        >
          <Cropper
            image={image}
            crop={crop}
            zoom={zoom}
            aspect={1}
            cropShape="rect"
            showGrid={true}
            onCropChange={onCropChange}
            onCropComplete={onCropAreaChange}
            onZoomChange={onZoomChange}
            classes={{
              cropAreaClassName: isLightImage ? 'use-dark-grid-area' : 'use-light-grid-area'
            }}
            onMediaLoaded={(_mediaSize) => {
              // Set a low minimum zoom to ensure full image accessibility
              setMinZoom(0.5);
              setZoom(1);
              
              // Analyze image brightness
              const img = new Image();
              img.onload = () => {
                const canvas = document.createElement('canvas');
                const ctx = canvas.getContext('2d');
                if (!ctx) {
                  console.error('Could not get canvas context');
                  return;
                }
                
                // Sample a smaller version for performance
                canvas.width = 50;
                canvas.height = 50;
                ctx.drawImage(img, 0, 0, 50, 50);
                
                const imageData = ctx.getImageData(0, 0, 50, 50);
                const data = imageData.data;
                let totalBrightness = 0;
                let pixelCount = 0;
                
                // Calculate average brightness
                for (let i = 0; i < data.length; i += 4) {
                  const r = data[i] ?? 0;
                  const g = data[i + 1] ?? 0;
                  const b = data[i + 2] ?? 0;
                  const a = data[i + 3] ?? 255;
                  
                  // Skip transparent pixels
                  if (a > 0) {
                    // Perceived brightness formula
                    const brightness = (0.299 * r + 0.587 * g + 0.114 * b);
                    totalBrightness += brightness;
                    pixelCount++;
                  }
                }
                
                const avgBrightness = pixelCount > 0 ? totalBrightness / pixelCount : 128;
                const isLight = avgBrightness > 128;
                
                console.log('Image brightness analysis:', {
                  avgBrightness,
                  isLight,
                  pixelCount,
                  className: isLight ? 'use-dark-grid' : 'use-light-grid'
                });
                
                setIsLightImage(isLight);
              };
              
              img.onerror = (e) => {
                console.error('Failed to load image for brightness analysis:', e);
              };
              
              // Handle both blob URLs and regular URLs
              img.src = image;
            }}
            minZoom={minZoom}
            maxZoom={5}
            restrictPosition={false}
            objectFit="horizontal-cover"
          />
          <button
            onClick={() => setCrop({ x: 0, y: 0 })}
            className="absolute bottom-3 right-3 p-2 text-white opacity-100 hover:opacity-90 transition-opacity duration-200"
          >
            <Crosshair className="h-5 w-5" />
          </button>
        </div>

        <div className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="zoom">Zoom</Label>
            <Slider
              id="zoom"
              min={minZoom}
              max={5}
              step={0.01}
              value={[zoom]}
              onValueChange={(value) => setZoom(value[0] ?? 1)}
              className="w-full"
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onCancel} disabled={isProcessing}>
            Cancel
          </Button>
          <Button 
            type="button"
            onClick={handleCrop} 
            disabled={isProcessing}
          >
            Crop Image
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}