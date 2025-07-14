#!/usr/bin/env python3
"""
Generate a 16MB test image with 160x40 RGB565 checkerboard pattern
"""

import struct
import os

def rgb888_to_rgb565(r, g, b):
    """Convert RGB888 to RGB565 format"""
    r5 = (r >> 3) & 0x1F
    g6 = (g >> 2) & 0x3F  
    b5 = (b >> 3) & 0x1F
    return (r5 << 11) | (g6 << 5) | b5

def generate_checkerboard_image():
    """Generate 160x40 RGB565 checkerboard pattern with 10x10 squares"""
    width = 160
    height = 40
    square_size = 10
    
    # Define colors for checkerboard (RGB888)
    colors = [
        (255, 0, 0),    # Red
        (0, 255, 0),    # Green
        (0, 0, 255),    # Blue
        (255, 255, 0),  # Yellow
        (255, 0, 255),  # Magenta
        (0, 255, 255),  # Cyan
        (255, 255, 255), # White
        (128, 128, 128), # Gray
    ]
    
    # Create image data
    image_data = bytearray()
    
    for y in range(height):
        for x in range(width):
            # Determine which square we're in
            square_x = x // square_size
            square_y = y // square_size
            
            # Calculate color index using checkerboard pattern
            color_index = (square_x + square_y) % len(colors)
            r, g, b = colors[color_index]
            
            # Convert to RGB565
            rgb565 = rgb888_to_rgb565(r, g, b)
            
            # Store as little-endian (LSB first)
            image_data.extend(struct.pack('<H', rgb565))
    
    return image_data

def generate_16mb_file():
    """Generate 16MB file with the test image repeated"""
    print("Generating 160x40 RGB565 checkerboard pattern...")
    
    # Generate the base image (160x40 pixels = 12800 bytes)
    base_image = generate_checkerboard_image()
    image_size = len(base_image)
    print(f"Base image size: {image_size} bytes ({160}x{40} RGB565)")
    
    # Target file size: 16MB
    target_size = 16 * 1024 * 1024
    print(f"Target file size: {target_size} bytes (16MB)")
    
    # Calculate how many times to repeat the image
    repeat_count = target_size // image_size
    remaining_bytes = target_size % image_size
    
    print(f"Will repeat image {repeat_count} times with {remaining_bytes} extra bytes")
    
    # Create output directory if it doesn't exist
    os.makedirs("tools/test_data", exist_ok=True)
    
    # Generate the 16MB file
    output_file = "tools/test_data/test_image_16mb.bin"
    print(f"Writing to {output_file}...")
    
    with open(output_file, 'wb') as f:
        # Write the complete images
        for i in range(repeat_count):
            f.write(base_image)
            if (i + 1) % 100 == 0:
                print(f"  Written {i + 1}/{repeat_count} images...")
        
        # Write remaining bytes (partial image)
        if remaining_bytes > 0:
            f.write(base_image[:remaining_bytes])
    
    print(f"✓ Generated {output_file} ({os.path.getsize(output_file)} bytes)")
    
    # Also generate just the single image for reference
    single_image_file = "tools/test_data/checkerboard_160x40.bin"
    with open(single_image_file, 'wb') as f:
        f.write(base_image)
    print(f"✓ Generated {single_image_file} ({len(base_image)} bytes)")
    
    return output_file, single_image_file

def print_image_info():
    """Print information about the generated image"""
    print("\n=== Image Information ===")
    print("Dimensions: 160x40 pixels")
    print("Format: RGB565 (16-bit per pixel)")
    print("Pattern: 10x10 pixel squares in checkerboard")
    print("Colors: Red, Green, Blue, Yellow, Magenta, Cyan, White, Gray")
    print("Total squares: 16x4 = 64 squares")
    print("Image size: 160 * 40 * 2 = 12,800 bytes")

if __name__ == "__main__":
    print("=== Test Image Generator ===")
    print_image_info()
    
    # Generate the files
    large_file, small_file = generate_16mb_file()
    
    print(f"\n=== Files Generated ===")
    print(f"16MB test file: {large_file}")
    print(f"Single image: {small_file}")
    print("\nReady for flashing to external Flash memory!")
