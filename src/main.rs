use clap::{App, Arg};
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::fs;
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;
use std::sync::atomic::{AtomicUsize, Ordering};

mod video_processor;
mod dash_generator;
mod utils;
mod json_generator;

use video_processor::VideoProcessor;
use dash_generator::DashGenerator;
use utils::{get_video_files, setup_output_dirs};
use json_generator::generate_server_json;

fn main() {
    let matches = App::new("视频DASH流转换工具")
        .version("1.0")
        .author("Rust AV1 转换工具")
        .about("将视频转换为DASH流格式，支持AV1编码")
        .arg(
            Arg::with_name("path")
                .short("i")
                .long("input")
                .value_name("路径")
                .help("要处理的视频文件夹路径")
                .required(true)
        )
        .arg(
            Arg::with_name("time")
                .short("t")
                .long("time")
                .value_name("秒数")
                .help("DASH分片的时间间隔（秒）")
                .default_value("10")
        )
        .arg(
            Arg::with_name("parallel")
                .short("p")
                .long("parallel")
                .value_name("线程数")
                .help("并行处理的线程数")
                .default_value("2")
        )
        .arg(
            Arg::with_name("serve")
                .long("serve")
                .value_name("服务器URLs")
                .help("服务器URLs列表，格式：[\"https://server1.com\",\"https://server2.com\"]")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("output")
                .long("output")
                .value_name("输出路径")
                .help("生成的DASH流文件的输出路径")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("copy")
                .long("copy")
                .value_name("是否复制")
                .help("是否复制而不是移动文件到输出路径")
                .takes_value(true)
                .default_value("true")
        )
        .get_matches();

    // 获取参数
    let input_path = matches.value_of("path").unwrap();
    let seg_duration = matches.value_of("time").unwrap().parse::<u32>().unwrap_or(10);
    let thread_count = matches.value_of("parallel").unwrap().parse::<usize>().unwrap_or(2);
    let output_path = matches.value_of("output").map(|p| PathBuf::from(p));
    let is_copy = matches.value_of("copy").unwrap_or("true") == "true";
    
    // 解析服务器URLs
    let servers = match matches.value_of("serve") {
        Some(servers_str) => {
            serde_json::from_str::<Vec<String>>(servers_str).unwrap_or_else(|_| {
                println!("{}", "❌ 服务器URLs格式错误，应为JSON数组".red());
                vec![]
            })
        },
        None => vec![],
    };

    println!("{}", "🚀 视频DASH流转换工具启动中...".green().bold());
    println!("{} {}", "📂 输入路径:".blue(), input_path);
    println!("{} {}秒", "⏱️ 分片时间:".blue(), seg_duration);
    println!("{} {}", "🧵 并行线程数:".blue(), thread_count);
    
    if !servers.is_empty() {
        println!("{}", "🌐 服务器URLs:".blue());
        for (i, server) in servers.iter().enumerate() {
            println!("   {}. {}", i+1, server);
        }
    }

    // 获取视频文件列表
    let video_files = get_video_files(input_path);
    if video_files.is_empty() {
        println!("{}", "❌ 未找到视频文件！".red().bold());
        return;
    }

    println!("{} {} 个视频文件", "🎬 找到:".green(), video_files.len());

    // 创建线程池
    let pool = ThreadPool::new(thread_count);
    let counter = Arc::new(AtomicUsize::new(0));
    let processed_videos = Arc::new(Mutex::new(Vec::new()));
    
    // 处理每个视频文件
    for video_path in video_files {
        let counter = counter.clone();
        let processed_videos = processed_videos.clone();
        // let servers = servers.clone();
        let output_path = output_path.clone();
        let is_copy = is_copy;
        let seg_duration = seg_duration;
        
        pool.execute(move || {
            let video_processor = VideoProcessor::new(&video_path);
            let file_name = video_processor.get_file_name();
            let thread_id = counter.fetch_add(1, Ordering::SeqCst) + 1;
            
            println!("{} [线程 {}] 开始处理: {}", "🔄".yellow(), thread_id, file_name);
            
            // 设置输出目录
            let (dash_dir, av1_dir, live_dir) = setup_output_dirs(&video_path);
            
            // 处理视频
            if let Some(processed_path) = video_processor.process(&av1_dir) {
                // 生成DASH流
                let dash_generator = DashGenerator::new(&processed_path, &dash_dir, seg_duration);
                if dash_generator.generate_dash(&live_dir) {
                    println!("{} [线程 {}] {} 处理完成", "✅".green(), thread_id, file_name);
                    
                    // 记录处理成功的视频
                    let mut videos = processed_videos.lock().unwrap();
                    videos.push((video_path.clone(), dash_dir.clone()));
                    
                    // 如果指定了输出路径，复制或移动文件
                    if let Some(ref out_path) = output_path {
                        let target_dir = out_path.join(Path::new(&dash_dir).file_name().unwrap());
                        
                        if is_copy {
                            println!("{} [线程 {}] 正在复制 {} 到 {}", "📋".blue(), thread_id, 
                                dash_dir.display(), target_dir.display());
                            match fs_extra::dir::copy(&dash_dir, out_path, &fs_extra::dir::CopyOptions::new()) {
                                Ok(_) => println!("{} [线程 {}] 复制成功", "✅".green(), thread_id),
                                Err(e) => println!("{} [线程 {}] 复制失败: {}", "❌".red(), thread_id, e),
                            }
                        } else {
                            println!("{} [线程 {}] 正在移动 {} 到 {}", "🚚".blue(), thread_id, 
                                dash_dir.display(), target_dir.display());
                            match fs::rename(&dash_dir, &target_dir) {
                                Ok(_) => println!("{} [线程 {}] 移动成功", "✅".green(), thread_id),
                                Err(e) => println!("{} [线程 {}] 移动失败: {}", "❌".red(), thread_id, e),
                            }
                        }
                    }
                } else {
                    println!("{} [线程 {}] {} DASH生成失败", "❌".red(), thread_id, file_name);
                }
            } else {
                println!("{} [线程 {}] {} 处理失败", "❌".red(), thread_id, file_name);
            }
        });
    }

    // 等待所有任务完成
    pool.join();

    // 生成服务器JSON文件
    if !servers.is_empty() {
        let processed_videos = processed_videos.lock().unwrap();
        generate_server_json(&processed_videos, &servers);
    }

    println!("{}", "🎉 所有视频处理完成！".green().bold());
}