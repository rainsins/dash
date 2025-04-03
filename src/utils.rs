use std::path::{Path, PathBuf};
use std::fs;
use colored::Colorize;
use walkdir::WalkDir;

// 获取指定文件夹中的所有视频文件
pub fn get_video_files(dir_path: &str) -> Vec<PathBuf> {
    let mut video_files = Vec::new();
    let video_extensions = ["mp4", "mkv", "avi", "mov", "webm", "flv", "wmv"];
    
    println!("{} 正在搜索视频文件...", "🔍".blue());
    
    for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if video_extensions.contains(&ext_str.as_str()) {
                    video_files.push(path.to_path_buf());
                }
            }
        }
    }
    
    video_files
}

// 设置输出目录结构
pub fn setup_output_dirs(video_path: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let parent_dir = video_path.parent().unwrap_or(Path::new("."));
    let file_stem = video_path.file_stem().unwrap_or_default().to_string_lossy();
    
    // 创建和视频同名的文件夹
    let dash_dir = parent_dir.join(&*file_stem);
    let av1_dir = dash_dir.join("av1");
    let live_dir = dash_dir.join("live");
    
    // 创建目录
    fs::create_dir_all(&dash_dir).unwrap_or_else(|e| {
        println!("{} 创建输出目录失败: {}", "❌".red(), e);
    });
    
    fs::create_dir_all(&av1_dir).unwrap_or_else(|e| {
        println!("{} 创建AV1目录失败: {}", "❌".red(), e);
    });
    
    fs::create_dir_all(&live_dir).unwrap_or_else(|e| {
        println!("{} 创建live目录失败: {}", "❌".red(), e);
    });
    
    (dash_dir, av1_dir, live_dir)
}