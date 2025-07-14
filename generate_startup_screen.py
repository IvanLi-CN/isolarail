#!/usr/bin/env python3

import struct

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

def generate_startup_screen():
    """生成启动屏图像数据"""
    print("=== 生成启动屏图像 ===")
    
    WIDTH = 160
    HEIGHT = 40
    
    startup_image = bytearray()
    
    print(f"生成{WIDTH}x{HEIGHT}启动屏图像...")
    
    for y in range(HEIGHT):
        for x in range(WIDTH):
            # 创建简单的渐变启动屏
            if y < 10:
                # 顶部区域 - 蓝色渐变
                intensity = (x * 255) // WIDTH
                color = rgb565_to_bytes(0, 0, intensity)
            elif y < 20:
                # 中上区域 - 绿色渐变  
                intensity = (x * 255) // WIDTH
                color = rgb565_to_bytes(0, intensity, 0)
            elif y < 30:
                # 中下区域 - 红色渐变
                intensity = (x * 255) // WIDTH
                color = rgb565_to_bytes(intensity, 0, 0)
            else:
                # 底部区域 - 白色渐变
                intensity = (x * 255) // WIDTH
                color = rgb565_to_bytes(intensity, intensity, intensity)
            
            startup_image.extend(color)
    
    # 保存启动屏图像
    with open('startup_screen.bin', 'wb') as f:
        f.write(startup_image)
    
    print("✓ 启动屏图像已生成: startup_screen.bin")
    print(f"  尺寸: {WIDTH}x{HEIGHT} 像素")
    print(f"  大小: {len(startup_image)} 字节")
    
    return len(startup_image)

def generate_combined_flash_data():
    """生成包含启动屏和棋盘图案的Flash数据"""
    print("\n=== 生成组合Flash数据 ===")
    
    WIDTH = 160
    HEIGHT = 40
    IMAGE_SIZE = WIDTH * HEIGHT * 2  # 每个图像的字节数
    
    # 生成启动屏 (已经生成)
    startup_size = generate_startup_screen()
    
    # 读取现有的棋盘图案
    try:
        with open('checkerboard_pattern.bin', 'rb') as f:
            checkerboard_data = f.read()
        print(f"✓ 读取棋盘图案: {len(checkerboard_data)} 字节")
    except FileNotFoundError:
        print("✗ 未找到棋盘图案文件，生成新的...")
        checkerboard_data = generate_checkerboard_pattern()
    
    # 读取启动屏数据
    with open('startup_screen.bin', 'rb') as f:
        startup_data = f.read()
    
    # 组合数据：启动屏在0x000000，棋盘图案在0x003200 (12800字节偏移)
    combined_data = bytearray()
    
    # 添加启动屏数据 (地址 0x000000)
    combined_data.extend(startup_data)
    
    # 填充到下一个图像位置 (0x003200 = 12800)
    padding_size = 12800 - len(startup_data)
    combined_data.extend(b'\x00' * padding_size)
    
    # 添加棋盘图案数据 (地址 0x003200)
    combined_data.extend(checkerboard_data[:IMAGE_SIZE])  # 只取一个图像的大小
    
    # 保存组合数据
    with open('combined_images.bin', 'wb') as f:
        f.write(combined_data)
    
    print(f"✓ 组合图像已生成: combined_images.bin")
    print(f"  启动屏地址: 0x000000 ({len(startup_data)} 字节)")
    print(f"  棋盘图案地址: 0x003200 ({IMAGE_SIZE} 字节)")
    print(f"  总大小: {len(combined_data)} 字节")

def generate_checkerboard_pattern():
    """生成棋盘图案作为备用"""
    WIDTH = 160
    HEIGHT = 40
    
    checkerboard_data = bytearray()
    
    # 10x10 棋盘图案
    colors = [
        rgb565_to_bytes(255, 0, 0),    # 红色
        rgb565_to_bytes(255, 255, 0),  # 黄色
        rgb565_to_bytes(0, 255, 0),    # 绿色
        rgb565_to_bytes(0, 255, 255),  # 青色
    ]
    
    for y in range(HEIGHT):
        for x in range(WIDTH):
            # 计算棋盘位置
            block_x = x // 16  # 每个方块16像素宽
            block_y = y // 4   # 每个方块4像素高
            
            # 选择颜色
            color_index = (block_x + block_y) % len(colors)
            color = colors[color_index]
            
            checkerboard_data.extend(color)
    
    # 保存棋盘图案
    with open('checkerboard_pattern.bin', 'wb') as f:
        f.write(checkerboard_data)
    
    print(f"✓ 生成棋盘图案: {len(checkerboard_data)} 字节")
    return checkerboard_data

if __name__ == "__main__":
    generate_combined_flash_data()
