use std::path::PathBuf;
use std::fs::File;
use std::io::Write;
use colored::Colorize;
use serde_json::json;

// 生成服务器JSON文件
pub fn generate_server_json(processed_videos: &[(PathBuf, PathBuf)], servers: &[String]) {
    if servers.is_empty() || processed_videos.is_empty() {
        return;
    }
    
    println!("{} 生成服务器JSON文件...", "📝".blue());
    
    for (i, server_url) in servers.iter().enumerate() {
        let server_num = i + 1;
        let filename = format!("server_{}.json", server_num);
        
        // 创建JSON数组
        let mut json_array = Vec::new();
        
        for (_video_path, dash_dir) in processed_videos {
            let video_name = dash_dir.file_name().unwrap_or_default().to_string_lossy();
            let mpd_url = format!("{}/{}/main.mpd", server_url, video_name);
            
            let video_entry = json!({
                "title": video_name,
                "url": mpd_url
            });
            
            json_array.push(video_entry);
        }
        
        // 写入JSON文件
        let json_str = serde_json::to_string_pretty(&json_array).unwrap_or_else(|e| {
            println!("{} JSON序列化失败: {}", "❌".red(), e);
            String::from("[]")
        });
        
        let mut file = match File::create(&filename) {
            Ok(file) => file,
            Err(e) => {
                println!("{} 创建JSON文件失败: {}", "❌".red(), e);
                continue;
            }
        };
        
        if let Err(e) = file.write_all(json_str.as_bytes()) {
            println!("{} 写入JSON文件失败: {}", "❌".red(), e);
        } else {
            println!("{} 成功创建JSON文件: {}", "✅".green(), filename);
        }
    }
}