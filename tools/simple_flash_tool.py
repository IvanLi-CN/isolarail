#!/usr/bin/env python3
"""
Simple Flash Programming Tool
Programs bitmap data to W25Q128 Flash via ST-Link
"""

import subprocess
import sys
import os

def main():
    print("=== Simple Flash Programming Tool ===")
    
    # Check if bitmap file exists
    bitmap_file = "screenshot_140x40.bin"
    if not os.path.exists(bitmap_file):
        print(f"Error: {bitmap_file} not found!")
        print("Please run png_to_bitmap_python.py first to generate the bitmap.")
        return 1
    
    # Get file size
    file_size = os.path.getsize(bitmap_file)
    print(f"Bitmap file: {bitmap_file}")
    print(f"File size: {file_size} bytes")
    
    # Use st-flash to program the bitmap to Flash address 0x000000
    try:
        print("Programming bitmap to Flash at address 0x000000...")
        
        # st-flash write <file> <address>
        cmd = ["st-flash", "write", bitmap_file, "0x000000"]
        print(f"Running: {' '.join(cmd)}")
        
        result = subprocess.run(cmd, capture_output=True, text=True)
        
        if result.returncode == 0:
            print("✓ Flash programming successful!")
            print("Output:", result.stdout)
        else:
            print("✗ Flash programming failed!")
            print("Error:", result.stderr)
            return 1
            
    except FileNotFoundError:
        print("Error: st-flash not found!")
        print("Please install st-link tools:")
        print("  macOS: brew install stlink")
        print("  Linux: sudo apt install stlink-tools")
        return 1
    except Exception as e:
        print(f"Error: {e}")
        return 1
    
    print("Flash programming complete!")
    return 0

if __name__ == "__main__":
    exit(main())
