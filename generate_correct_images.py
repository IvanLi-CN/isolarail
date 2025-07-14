#!/usr/bin/env python3

import struct
from PIL import Image
import os

def rgb565_to_bytes(r, g, b):
    """将RGB值转换为RGB565格式的小端字节"""
    # 将8位RGB转换为RGB565
    r5 = (r >> 3) & 0x1F  # 5位红色
    g6 = (g >> 2) & 0x3F  # 6位绿色  
    b5 = (b >> 3) & 0x1F  # 5位蓝色
    
    # 组合成16位RGB565值
    rgb565 = (r5 << 11) | (g6 << 5) | b5
    
    # 转换为小端字节序
    return struct.pack('<H', rgb565)

def convert_png_to_rgb565(png_path, output_path):
    """将PNG图像转换为RGB565格式的二进制文件"""
    print(f"转换PNG图像: {png_path}")
    
    # 打开PNG图像
    img = Image.open(png_path)
    
    # 确保图像是RGB模式
    if img.mode != 'RGB':
        img = img.convert('RGB')
    
    # 检查尺寸
    width, height = img.size
    print(f"图像尺寸: {width}x{height}")
    
    if width != 160 or height != 40:
        print(f"警告: 图像尺寸不是160x40，将调整大小")
        img = img.resize((160, 40), Image.Resampling.LANCZOS)
    
    # 转换为RGB565二进制数据
    rgb565_data = bytearray()
    
    for y in range(40):
        for x in range(160):
            r, g, b = img.getpixel((x, y))
            rgb565_bytes = rgb565_to_bytes(r, g, b)
            rgb565_data.extend(rgb565_bytes)
    
    # 保存二进制文件
    with open(output_path, 'wb') as f:
        f.write(rgb565_data)
    
    print(f"✓ 启动屏转换完成: {output_path}")
    print(f"  大小: {len(rgb565_data)} 字节")
    
    return len(rgb565_data)

def generate_10x10_checkerboard():
    """生成10x10彩色方块棋盘图案"""
    print("生成10x10彩色方块棋盘图案...")
    
    WIDTH = 160
    HEIGHT = 40
    
    # 10x10方块，每个方块16x4像素
    BLOCK_WIDTH = 16  # 160/10 = 16像素
    BLOCK_HEIGHT = 4  # 40/10 = 4像素
    
    # 定义10种不同的颜色
    colors = [
        (255, 0, 0),    # 红色
        (255, 128, 0),  # 橙色
        (255, 255, 0),  # 黄色
        (128, 255, 0),  # 黄绿色
        (0, 255, 0),    # 绿色
        (0, 255, 128),  # 青绿色
        (0, 255, 255),  # 青色
        (0, 128, 255),  # 蓝青色
        (0, 0, 255),    # 蓝色
        (128, 0, 255),  # 紫色
    ]
    
    checkerboard_data = bytearray()
    
    for y in range(HEIGHT):
        for x in range(WIDTH):
            # 计算当前像素属于哪个方块
            block_x = x // BLOCK_WIDTH  # 0-9
            block_y = y // BLOCK_HEIGHT  # 0-9
            
            # 使用棋盘模式选择颜色
            color_index = (block_x + block_y) % len(colors)
            r, g, b = colors[color_index]
            
            # 转换为RGB565
            rgb565_bytes = rgb565_to_bytes(r, g, b)
            checkerboard_data.extend(rgb565_bytes)
    
    # 保存棋盘图案
    with open('checkerboard_10x10.bin', 'wb') as f:
        f.write(checkerboard_data)
    
    print(f"✓ 10x10棋盘图案生成完成: checkerboard_10x10.bin")
    print(f"  尺寸: 160x40像素，10x10方块")
    print(f"  每个方块: {BLOCK_WIDTH}x{BLOCK_HEIGHT}像素")
    print(f"  大小: {len(checkerboard_data)} 字节")
    print(f"  颜色数量: {len(colors)}种")
    
    return checkerboard_data

def generate_combined_flash_data():
    """生成包含正确启动屏和10x10棋盘的Flash数据"""
    print("\n=== 生成正确的组合Flash数据 ===")
    
    IMAGE_SIZE = 160 * 40 * 2  # 每个图像的字节数 (12800字节)
    
    # 1. 转换PNG启动屏
    png_path = "assets/images/screenshots/screenshot_160x40.png"
    if not os.path.exists(png_path):
        print(f"错误: 找不到启动屏文件 {png_path}")
        return
    
    startup_size = convert_png_to_rgb565(png_path, "startup_screen_correct.bin")
    
    # 2. 生成10x10棋盘图案
    checkerboard_data = generate_10x10_checkerboard()
    
    # 3. 读取启动屏数据
    with open('startup_screen_correct.bin', 'rb') as f:
        startup_data = f.read()
    
    # 4. 组合数据：启动屏在0x000000，棋盘图案在0x003200 (12800字节偏移)
    combined_data = bytearray()
    
    # 添加启动屏数据 (地址 0x000000)
    combined_data.extend(startup_data)
    
    # 确保启动屏数据正好是12800字节
    if len(startup_data) < IMAGE_SIZE:
        padding_size = IMAGE_SIZE - len(startup_data)
        combined_data.extend(b'\x00' * padding_size)
        print(f"启动屏数据填充了 {padding_size} 字节")
    elif len(startup_data) > IMAGE_SIZE:
        combined_data = combined_data[:IMAGE_SIZE]
        print(f"启动屏数据截断到 {IMAGE_SIZE} 字节")
    
    # 添加棋盘图案数据 (地址 0x003200)
    combined_data.extend(checkerboard_data[:IMAGE_SIZE])  # 只取一个图像的大小
    
    # 保存组合数据
    with open('combined_images_correct.bin', 'wb') as f:
        f.write(combined_data)
    
    print(f"\n✓ 正确的组合图像已生成: combined_images_correct.bin")
    print(f"  启动屏地址: 0x000000 ({IMAGE_SIZE} 字节) - 来自PNG文件")
    print(f"  棋盘图案地址: 0x003200 ({IMAGE_SIZE} 字节) - 10x10彩色方块")
    print(f"  总大小: {len(combined_data)} 字节")

if __name__ == "__main__":
    generate_combined_flash_data()
