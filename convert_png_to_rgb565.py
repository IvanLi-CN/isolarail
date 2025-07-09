#!/usr/bin/env python3
"""
Convert PNG image to RGB565 raw binary format for embedded display
"""

import sys
from PIL import Image
import struct

def rgb888_to_rgb565(r, g, b):
    """Convert RGB888 to RGB565 format"""
    # Convert 8-bit values to 5/6/5 bit values
    r5 = (r >> 3) & 0x1F  # 5 bits for red
    g6 = (g >> 2) & 0x3F  # 6 bits for green  
    b5 = (b >> 3) & 0x1F  # 5 bits for blue
    
    # Pack into 16-bit value: RRRRR GGGGGG BBBBB
    rgb565 = (r5 << 11) | (g6 << 5) | b5
    return rgb565

def convert_png_to_rgb565(input_file, output_file):
    """Convert PNG image to RGB565 binary format"""
    try:
        # Open and convert image to RGB
        img = Image.open(input_file)
        img = img.convert('RGB')
        
        width, height = img.size
        print(f"Image size: {width}x{height}")
        
        # Create binary data
        binary_data = bytearray()
        
        # Process pixel by pixel
        for y in range(height):
            for x in range(width):
                r, g, b = img.getpixel((x, y))
                rgb565 = rgb888_to_rgb565(r, g, b)
                
                # Pack as little-endian 16-bit value
                binary_data.extend(struct.pack('<H', rgb565))
        
        # Write binary data to file
        with open(output_file, 'wb') as f:
            f.write(binary_data)
        
        print(f"Converted {input_file} to {output_file}")
        print(f"Output size: {len(binary_data)} bytes")
        print(f"Expected size: {width * height * 2} bytes")
        
        return True
        
    except Exception as e:
        print(f"Error converting image: {e}")
        return False

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python3 convert_png_to_rgb565.py <input.png> <output.bin>")
        sys.exit(1)
    
    input_file = sys.argv[1]
    output_file = sys.argv[2]
    
    if convert_png_to_rgb565(input_file, output_file):
        print("Conversion successful!")
    else:
        print("Conversion failed!")
        sys.exit(1)
