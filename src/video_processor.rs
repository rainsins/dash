use std::path::{Path, PathBuf};
use std::process::Command;
use colored::Colorize;
use std::fs;

pub struct VideoProcessor {
    video_path: PathBuf,
}

impl VideoProcessor {
    pub fn new(video_path: &Path) -> Self {
        VideoProcessor {
            video_path: PathBuf::from(video_path),
        }
    }

    pub fn get_file_name(&self) -> String {
        self.video_path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }

    // 处理视频，返回处理后的视频路径（用于后续生成DASH）
    pub fn process(&self, av1_dir: &Path) -> Option<PathBuf> {
        // 确保输出目录存在
        if !av1_dir.exists() {
            fs::create_dir_all(av1_dir).unwrap_or_else(|e| {
                println!("{} 创建AV1目录失败: {}", "❌".red(), e);
            });
        }

        let file_name = self.get_file_name();
        let out_file = av1_dir.join(&file_name);

        // 检查视频编码
        if self.is_av1_encoded() {
            println!("{} {} 已经是AV1编码, 直接复制", "ℹ️".blue(), file_name);
            if let Err(e) = fs::copy(&self.video_path, &out_file) {
                println!("{} 复制AV1视频失败: {}", "❌".red(), e);
                return None;
            }
            return Some(out_file);
        } else {
            // 需要转码为AV1
            println!("{} {} 不是AV1编码, 开始转码", "🔄".yellow(), file_name);
            if self.transcode_to_av1(&out_file) {
                println!("{} {} 转码为AV1成功", "✅".green(), file_name);
                return Some(out_file);
            }
        }

        None
    }

    // 检查视频是否是AV1编码
    fn is_av1_encoded(&self) -> bool {
        let output = Command::new("ffprobe")
            .args(&[
                "-v", "error",
                "-select_streams", "v:0",
                "-show_entries", "stream=codec_name",
                "-of", "default=noprint_wrappers=1:nokey=1",
                self.video_path.to_str().unwrap(),
            ])
            .output();

        match output {
            Ok(output) => {
                let codec = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
                println!("{} 检测到视频编码: {}", "🔍".blue(), codec);
                codec == "av1"
            },
            Err(e) => {
                println!("{} 检测视频编码失败: {}", "❌".red(), e);
                false
            }
        }
    }

    // 使用QSVEncC64转码视频为AV1格式
    fn transcode_to_av1(&self, output_path: &Path) -> bool {
        println!("{} 正在使用QSVEncC64转码为AV1: {}", "🛠️".yellow(), output_path.display());
        
        // 使用QSVEncC64进行转码，多级编码回退
        let result = Command::new("QSVEncC64")
            .args(&[
                "--codec", "av1",
                "--input", self.video_path.to_str().unwrap(),
                "--output", output_path.to_str().unwrap(),
                "--quality", "balanced",
                "--fallback-rc",  // 启用多级编码回退
            ])
            .status();

        match result {
            Ok(status) => {
                if status.success() {
                    // 检查转码后的文件是否有视频和音频流
                    self.check_streams(output_path)
                } else {
                    println!("{} QSVEncC64转码失败，退出码: {:?}", "❌".red(), status.code());
                    
                    // 如果QSVEncC64失败，尝试使用ffmpeg作为备选
                    println!("{} 尝试使用ffmpeg进行备选转码", "🔄".yellow());
                    let ffmpeg_result = Command::new("ffmpeg")
                        .args(&[
                            "-i", self.video_path.to_str().unwrap(),
                            "-c:v", "libaom-av1",
                            "-crf", "30",
                            "-b:v", "0",
                            "-c:a", "copy",
                            output_path.to_str().unwrap(),
                        ])
                        .status();
                        
                    match ffmpeg_result {
                        Ok(ffmpeg_status) => {
                            if ffmpeg_status.success() {
                                println!("{} ffmpeg转码成功", "✅".green());
                                self.check_streams(output_path)
                            } else {
                                println!("{} ffmpeg转码失败，退出码: {:?}", "❌".red(), ffmpeg_status.code());
                                false
                            }
                        },
                        Err(e) => {
                            println!("{} 执行ffmpeg失败: {}", "❌".red(), e);
                            false
                        }
                    }
                }
            },
            Err(e) => {
                println!("{} 执行QSVEncC64失败: {}", "❌".red(), e);
                false
            }
        }
    }

    // 检查转码后的文件是否有视频和音频流
    fn check_streams(&self, file_path: &Path) -> bool {
        println!("{} 检查转码后的流...", "🔍".blue());
        
        let video_check = Command::new("ffprobe")
            .args(&[
                "-v", "error",
                "-select_streams", "v:0",
                "-count_packets",
                "-show_entries", "stream=nb_read_packets",
                "-of", "csv=p=0",
                file_path.to_str().unwrap(),
            ])
            .output();
            
        let audio_check = Command::new("ffprobe")
            .args(&[
                "-v", "error",
                "-select_streams", "a:0",
                "-count_packets",
                "-show_entries", "stream=nb_read_packets",
                "-of", "csv=p=0",
                file_path.to_str().unwrap(),
            ])
            .output();
        
        let has_video = match video_check {
            Ok(output) => {
                let packets = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let count = packets.parse::<i32>().unwrap_or(0);
                if count > 0 {
                    println!("{} 检测到视频流: {} 个数据包", "✅".green(), count);
                    true
                } else {
                    println!("{} 没有检测到视频流", "⚠️".yellow());
                    false
                }
            },
            Err(e) => {
                println!("{} 检查视频流失败: {}", "❌".red(), e);
                false
            }
        };
        
        let has_audio = match audio_check {
            Ok(output) => {
                let packets = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let count = packets.parse::<i32>().unwrap_or(0);
                if count > 0 {
                    println!("{} 检测到音频流: {} 个数据包", "✅".green(), count);
                    true
                } else {
                    println!("{} 没有检测到音频流", "⚠️".yellow());
                    // 音频可能没有，所以这里不返回false
                    true
                }
            },
            Err(e) => {
                println!("{} 检查音频流失败: {}", "❌".red(), e);
                false
            }
        };
        
        has_video && has_audio
    }
}