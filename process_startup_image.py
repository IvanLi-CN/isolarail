#!/usr/bin/env python3
"""
启动屏图片处理工具
将screenshot-De8lylrp.png转换为160x40的RGB565位图数据
"""

from PIL import Image
import struct
import os

def process_startup_image():
    """处理启动屏图片"""
    
    # 1. 加载原始图片
    img_path = 'assets/images/screenshots/screenshot-De8lylrp.png'
    if not os.path.exists(img_path):
        print(f"错误: 图片文件不存在: {img_path}")
        return
    
    img = Image.open(img_path)
    print(f'原始图片尺寸: {img.size[0]} x {img.size[1]}')
    print(f'图片模式: {img.mode}')
    
    # 2. 计算160:40的比例 (4:1) 并裁剪
    target_ratio = 160 / 40  # 4:1
    current_ratio = img.size[0] / img.size[1]
    
    print(f'目标比例: {target_ratio:.2f}:1')
    print(f'当前比例: {current_ratio:.2f}:1')
    
    # 计算裁剪区域 (居中裁剪)
    if current_ratio > target_ratio:
        # 图片太宽，需要裁剪宽度
        new_width = int(img.size[1] * target_ratio)
        new_height = img.size[1]
        left = (img.size[0] - new_width) // 2
        top = 0
        right = left + new_width
        bottom = new_height
    else:
        # 图片太高，需要裁剪高度
        new_width = img.size[0]
        new_height = int(img.size[0] / target_ratio)
        left = 0
        top = (img.size[1] - new_height) // 2
        right = new_width
        bottom = top + new_height
    
    print(f'裁剪区域: ({left}, {top}, {right}, {bottom})')
    print(f'裁剪后尺寸: {right-left} x {bottom-top}')
    
    # 执行裁剪
    cropped = img.crop((left, top, right, bottom))
    cropped_path = 'assets/images/screenshots/screenshot_cropped_4_1.png'
    cropped.save(cropped_path)
    print(f'已保存裁剪后的图片: {cropped_path}')
    
    # 3. 缩放到160x40像素
    resized = cropped.resize((160, 40), Image.Resampling.LANCZOS)
    resized_path = 'assets/images/screenshots/screenshot_160x40.png'
    resized.save(resized_path)
    print(f'已保存缩放后的图片: {resized_path}')
    
    # 4. 转换为RGB565格式
    # 确保图片是RGB模式
    if resized.mode != 'RGB':
        resized = resized.convert('RGB')
    
    # 转换为RGB565位图数据
    rgb565_data = []
    for y in range(40):
        for x in range(160):
            r, g, b = resized.getpixel((x, y))
            
            # 转换为RGB565格式
            # R: 5位 (31-27), G: 6位 (26-21), B: 5位 (20-16)
            r5 = (r >> 3) & 0x1F
            g6 = (g >> 2) & 0x3F  
            b5 = (b >> 3) & 0x1F
            
            rgb565 = (r5 << 11) | (g6 << 5) | b5
            
            # 小端序存储
            rgb565_data.append(rgb565 & 0xFF)        # 低字节
            rgb565_data.append((rgb565 >> 8) & 0xFF) # 高字节
    
    # 保存RGB565位图数据
    bitmap_path = 'startup_bitmap_160x40.bin'
    with open(bitmap_path, 'wb') as f:
        f.write(bytes(rgb565_data))
    
    print(f'已保存RGB565位图数据: {bitmap_path}')
    print(f'位图数据大小: {len(rgb565_data)} 字节 (预期: {160*40*2} 字节)')
    
    # 5. 创建16Mbit (2MB) bin文件，将位图数据放在页对齐位置
    # W25Q128的页大小是256字节，扇区大小是4KB
    flash_size = 16 * 1024 * 1024  # 16MB
    flash_data = bytearray(flash_size)
    
    # 将位图数据放在地址0x000000 (已经是页对齐的)
    bitmap_start_addr = 0x000000
    flash_data[bitmap_start_addr:bitmap_start_addr + len(rgb565_data)] = rgb565_data
    
    # 保存完整的flash bin文件
    flash_bin_path = 'startup_flash_16mb.bin'
    with open(flash_bin_path, 'wb') as f:
        f.write(flash_data)
    
    print(f'已创建16MB flash bin文件: {flash_bin_path}')
    print(f'位图数据位置: 0x{bitmap_start_addr:06X} - 0x{bitmap_start_addr + len(rgb565_data) - 1:06X}')
    
    return bitmap_path, flash_bin_path

if __name__ == '__main__':
    process_startup_image()
