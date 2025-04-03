# Video2Dash - 视频转Dash流工具

## 简介 🌟

Video2Dash是一个强大的命令行工具，专为将视频批量转换为DASH流格式而设计，支持AV1编码优化。该工具会自动遍历指定文件夹中的所有视频，并为每个视频创建相应的DASH流文件，便于在浏览器中通过dash.js和artplayer.js播放。

## 缘起 🤡

- 入手了一块A770，想让它干点事。
- 老师的视频太大了，动辄5-6G，转成av1的节省空间。
- ffmpeg总是跑不满这张卡，所以用QSVEncC来进行转码。
- 学习Rust的使用和AI的使用，其实就是AI（claude）生成的😀，我就小小地改了一丢丢。
- 这个readme页面都是AI生成的。

## 特性 ✨

- 🔍 自动遍历文件夹中的所有视频文件
- 🎬 检测并处理AV1编码视频
- 🔄 使用Intel QSV硬件加速将非AV1视频转码为AV1格式
- 📊 生成标准DASH流(.mpd和.m4s文件)
- 📝 生成包含视频列表的JSON文件，便于前端集成
- 🧵 支持多线程并行处理，提高效率
- 🔧 高度可定制的参数选项
- 📈 详细的日志输出，方便调试
- 🎨 彩色终端输出，信息一目了然

## 系统要求 🖥️

- 操作系统: Windows 10/11
- CPU: 支持Intel Quick Sync Video的处理器（如9600K）
- GPU: 支持视频加速的显卡（如Intel Arc A770）
- 内存: 8GB+（推荐32GB）
- 依赖工具:
  - FFmpeg
  - QSVEncC64

## 安装 📦

1. 从[发布页面](https://github.com/yourusername/video2dash/releases)下载最新版本的Video2Dash.exe
2. 将可执行文件放置在系统PATH中，或直接在文件所在目录使用

## 使用方法 🚀

### 基本用法

```bash
video2dash.exe [选项] <输入路径>
```

### 命令行选项

| 选项 | 描述 | 默认值 |
|------|------|--------|
| `<输入路径>` | 要处理的视频文件夹路径 | 必须指定 |
| `-t, --time <seconds>` | 分片时间间隔(秒) | 10 |
| `-p, --parallel <num>` | 并行处理的线程数 | 2 |
| `-o, --output <path>` | 输出文件夹路径 | 与输入相同 |
| `-c, --copy <bool>` | 复制而非移动生成的文件 | true |
| `-s, --serve <urls>` | 指定服务器URL列表 | [] |
| `-h, --help` | 显示帮助信息 | - |
| `-v, --version` | 显示版本信息 | - |

### 示例

**基本转换:**

```bash
video2dash.exe D:\Videos
```

**指定分片时间和并行线程:**

```bash
video2dash.exe -t 8 -p 4 D:\Videos
```

**自定义服务器URL:**

```bash
video2dash.exe --serve ["https://server1.com","https://server2.com"] D:\Videos
```

**将生成的文件复制到指定位置:**

```bash
video2dash.exe --output E:\Converted --copy true D:\Videos
```

## 工作流程 🔄

1. 遍历指定文件夹中的所有视频文件
2. 对每个视频:
   - 检查是否为AV1编码
   - 非AV1视频使用QSVEncC64转码为AV1格式
   - 使用FFmpeg生成DASH流文件
   - 创建文件夹结构并存放生成的文件
3. 为每个指定的服务器生成JSON索引文件

## 输出结构 📁

对于每个名为`video-name.mp4`的视频文件，工具将生成:

```txt
video-name/
├── main.mpd                 # DASH清单文件
├── av1/
│   └── video_av1.mp4        # AV1编码的视频文件
└── live/
    ├── init_video_0.mp4     # 初始化片段
    ├── chunk_video_0_1.m4s  # 视频片段
    ├── chunk_video_0_2.m4s
    └── ...
```

同时在处理目录下生成`server_1.json`、`server_2.json`等文件，包含所有视频的信息。

## 故障排除 🔧

### 常见问题

1. **问题**: 转码过程中出现错误  
   **解决方案**: 检查日志输出，确认视频文件未损坏，并确保系统支持Intel QSV

2. **问题**: 生成的DASH流无法播放  
   **解决方案**: 检查生成的.mpd文件路径是否正确，确保web服务器配置了正确的MIME类型

3. **问题**: 占用内存过多  
   **解决方案**: 减少并行处理线程数(`-p`参数)

## 开发笔记 📝

- 使用Rust的并发特性确保高效处理
- FFmpeg和QSVEncC64命令通过进程调用实现
- 路径处理统一使用POSIX风格(`/`)
- 彩色输出使用`colored`库实现
- 模块化设计便于维护和扩展

## 参与贡献 🤝

欢迎提交问题报告和功能请求! 如果您想贡献代码，请先创建issue讨论您的想法。

## 许可证 📄

MIT License

---

⭐ 如果您觉得这个工具有用，请考虑给它一个星标! ⭐
