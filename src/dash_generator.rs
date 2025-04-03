use colored::Colorize;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use regex::Regex;

pub struct DashGenerator {
    video_path: PathBuf,
    dash_dir: PathBuf,
    seg_duration: u32,
}

impl DashGenerator {
    pub fn new(video_path: &Path, dash_dir: &Path, seg_duration: u32) -> Self {
        DashGenerator {
            video_path: PathBuf::from(video_path),
            dash_dir: PathBuf::from(dash_dir),
            seg_duration,
        }
    }

    // 生成DASH流
    pub fn generate_dash(&self, live_dir: &Path) -> bool {
        let video_name = self.video_path.file_name().unwrap().to_string_lossy();
        println!("{} 为 {} 生成DASH流...", "🔄".yellow(), video_name);

        // 确保live目录存在
        if !live_dir.exists() {
            if let Err(e) = fs::create_dir_all(live_dir) {
                println!("{} 创建live目录失败: {}", "❌".red(), e);
                return false;
            }
        }

        // MPD文件路径
        let mpd_path = self.dash_dir.join("main.mpd");
        let video_path = self.dash_dir.join("live");

        println!("{} 使用ffmpeg生成DASH流", "🛠️".blue());

        let output = Command::new("ffmpeg").args(&[
            "-i",
            self.video_path.to_str().unwrap(),
            "-v", 
            "level+debug",
            "-c",
            "copy",
            "-map",
            "0",
            "-f",
            "dash",
            "-seg_duration",
            &self.seg_duration.to_string(),
            "-use_template",
            "1",
            "-use_timeline",
            "1",
            "-dash_segment_type",
            "mp4",
            "-init_seg_name",
            &format!(
                "{}/init_$RepresentationID$.m4s",
                video_path.to_str().unwrap()
            ),
            "-media_seg_name",
            &format!(
                "{}/chunk_$RepresentationID$_$Number$.m4s",
                video_path.to_str().unwrap()
            ),
            mpd_path.to_str().unwrap(),
        ]).output();


        match output {
            Ok(output) => {
                if output.status.success() {
                    println!("{} DASH流生成成功: {}", "✅".green(), mpd_path.display());

                    // 修复MPD文件中的路径
                    if self.fix_mpd_paths(&mpd_path) {
                        true
                    } else {
                        println!("{} 修复MPD文件路径失败", "❌".red());
                        false
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    println!("{} DASH流生成失败: {}", "❌".red(), stderr);
                    false
                }
            }
            Err(e) => {
                println!("{} 执行ffmpeg失败: {}", "❌".red(), e);
                false
            }
        }
    }

    // 修复MPD文件中的路径
    fn fix_mpd_paths(&self, mpd_path: &Path) -> bool {
        println!("{} 修复MPD文件中的路径...", "🔧".yellow());

        // 读取MPD文件内容
        let mut file = match fs::File::open(mpd_path) {
            Ok(file) => file,
            Err(e) => {
                println!("{} 打开MPD文件失败: {}", "❌".red(), e);
                return false;
            }
        };

        let mut content = String::new();
        if let Err(e) = file.read_to_string(&mut content) {
            println!("{} 读取MPD文件失败: {}", "❌".red(), e);
            return false;
        }

        fn fix_xml_paths(xml: &str) -> String {
            // 匹配所有包含 ./.../live/ 模式的属性值
            let re = Regex::new(r#"\./([^/"]+/)?live/([^"]+)"#).unwrap();
            
            re.replace_all(xml, |caps: &regex::Captures| {
                format!("live/{}", &caps[2])
            }).to_string()
        }

        let p_content = content.replace("\\", "/");

        // 替换路径，确保使用正确的格式
        // 直接替换路径模式，不依赖于具体的文件夹名称
        // 将形如 "./video-name/live/" 修改为 "live/"

        // 进行所有替换
        let fixed_content = fix_xml_paths(&p_content);

        // 如果内容有变化，写回文件
        if content != fixed_content {
            let mut file = match fs::File::create(mpd_path) {
                Ok(file) => file,
                Err(e) => {
                    println!("{} 创建MPD文件失败: {}", "❌".red(), e);
                    return false;
                }
            };

            if let Err(e) = file.write_all(fixed_content.as_bytes()) {
                println!("{} 写入MPD文件失败: {}", "❌".red(), e);
                return false;
            }

            println!("{} MPD文件路径已修复", "✅".green());
        } else {
            println!("{} MPD文件路径无需修改", "✅".green());
        }

        true
    }
}
