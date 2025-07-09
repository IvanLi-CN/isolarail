#!/usr/bin/env python3
"""
PNG to 140×40 Bitmap Converter
Converts a PNG image to 140×40 RGB565 bitmap format
"""

import struct
from PIL import Image

def rgb888_to_rgb565(r, g, b):
    """Convert RGB888 to RGB565"""
    r5 = (r >> 3) & 0x1F
    g6 = (g >> 2) & 0x3F
    b5 = (b >> 3) & 0x1F
    return (r5 << 11) | (g6 << 5) | b5

def main():
    print("Converting PNG to 140×40 bitmap...")
    
    # Load and resize PNG
    try:
        img = Image.open("screenshot-De8lylrp.png")
        print(f"Original image: {img.size[0]}×{img.size[1]}")
        
        # Resize to 140×40 using high-quality resampling
        resized = img.resize((140, 40), Image.Resampling.LANCZOS)
        print(f"Resized to: {resized.size[0]}×{resized.size[1]}")
        
        # Convert to RGB (remove alpha if present)
        rgb_img = resized.convert('RGB')
        
        # Convert to RGB565 bitmap
        bitmap_data = bytearray()
        
        for y in range(40):
            for x in range(140):
                r, g, b = rgb_img.getpixel((x, y))
                rgb565 = rgb888_to_rgb565(r, g, b)
                
                # Store as little-endian
                bitmap_data.extend(struct.pack('<H', rgb565))
        
        # Create bitmap header
        signature = 0x424D5447  # "GTMB" signature
        width = 140
        height = 40
        format_type = 1  # RGB565 format
        data_size = len(bitmap_data)
        
        # Calculate simple checksum
        checksum = 0
        for i in range(0, len(bitmap_data), 4):
            chunk = bitmap_data[i:i+4]
            if len(chunk) == 4:
                checksum = (checksum + struct.unpack('<I', chunk)[0]) & 0xFFFFFFFF
            else:
                # Handle remaining bytes
                padded = chunk + b'\x00' * (4 - len(chunk))
                checksum = (checksum + struct.unpack('<I', padded)[0]) & 0xFFFFFFFF
        
        # Write bitmap file
        with open("screenshot_140x40.bin", "wb") as f:
            # Write header
            f.write(struct.pack('<I', signature))
            f.write(struct.pack('<I', width))
            f.write(struct.pack('<I', height))
            f.write(struct.pack('<I', format_type))
            f.write(struct.pack('<I', data_size))
            f.write(struct.pack('<I', checksum))
            
            # Write bitmap data
            f.write(bitmap_data)
        
        print("Generated screenshot_140x40.bin")
        print(f"Size: {24 + len(bitmap_data)} bytes")  # 24 bytes header + data
        print(f"Dimensions: {width}×{height}")
        print("Format: RGB565")
        print(f"Checksum: 0x{checksum:08X}")
        
    except Exception as e:
        print(f"Error: {e}")
        return 1
    
    return 0

if __name__ == "__main__":
    exit(main())
