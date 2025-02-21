use std::path::PathBuf;
use std::fs;
use std::process::Command;
use tauri::Manager;
use rand::seq::SliceRandom; // 导入 trait
use rand::thread_rng;       // 导入随机数生成器
use std::process::Output;
use serde_json;
use tokio::task;
use futures::future::join_all;
use std::sync::Arc;
use tauri::Emitter;
use num_cpus;
use threadpool::ThreadPool;
use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use anyhow::{Result, Context};

// 定义处理状态结构
struct ProcessingState {
    total_files: AtomicUsize,
    processed_files: AtomicUsize,
    failed_files: DashMap<String, String>,
    output_files: DashMap<usize, String>,
    current_progress: AtomicUsize,  // 添加当前总进度
}

#[tauri::command]
pub async fn start_editing(
    app: tauri::AppHandle,
    input_path: String,
    audio_path: String,
    is_folder: bool,
    order: Option<String>,
    duration: u32,
) -> Result<Vec<String>, String> {
    println!("开始处理视频编辑任务");

    // 创建处理状态
    let state = Arc::new(ProcessingState {
        total_files: AtomicUsize::new(0),
        processed_files: AtomicUsize::new(0),
        failed_files: DashMap::new(),
        output_files: DashMap::new(),
        current_progress: AtomicUsize::new(0),
    });

    // 创建线程池
    let pool = ThreadPool::new(num_cpus::get());
    let batch_size = 100; // 增加批处理大小
    
    // 创建输出目录
    let output_dir = create_output_dir(&PathBuf::from(&input_path))?;
    let output_dir = Arc::new(output_dir);

    // 音频文件路径
    let audio_path = Arc::new(PathBuf::from(audio_path));
    
    if is_folder {
        // 使用 rayon 并行收集文件
        let video_files: Vec<_> = fs::read_dir(&input_path)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .filter(|entry| {
                let path = entry.path();
                path.is_file() && matches!(
                    path.extension().and_then(|s| s.to_str()),
                    Some("mp4" | "mov" | "avi" | "wmv" | "flv" | "mkv")
                )
            })
            .map(|entry| entry.path())
            .collect();

        // 更新总文件数
        state.total_files.store(video_files.len(), Ordering::SeqCst);

        // 根据需要随机化文件顺序
        let mut video_files = video_files;
        if let Some("random") = order.as_deref() {
            video_files.shuffle(&mut rand::thread_rng());
        }

        // 分批处理文件
        for (batch_index, chunk) in video_files.chunks(batch_size).enumerate() {
            let chunk_files = chunk.to_vec();
            let state = Arc::clone(&state);
            let output_dir = Arc::clone(&output_dir);
            let audio_path = Arc::clone(&audio_path);
            let app = app.clone();

            pool.execute(move || {
                process_batch(
                    chunk_files,
                    batch_index,
                    duration,
                    &output_dir,
                    &audio_path,
                    &state,
                    app,
                );
            });
        }

        // 等待所有任务完成
        pool.join();

        // 收集输出文件
        let mut output_files: Vec<String> = state
            .output_files
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
        output_files.sort();

        // 报告处理结果
        println!(
            "处理完成。成功: {}, 失败: {}, 总计: {}",
            state.processed_files.load(Ordering::SeqCst),
            state.failed_files.len(),
            state.total_files.load(Ordering::SeqCst)
        );

        Ok(output_files)
    } else {
        println!("开始处理单个视频文件");
        let input_path = PathBuf::from(&input_path);
        
        // 验证文件
        if !input_path.exists() || input_path.extension().and_then(|s| s.to_str()) != Some("mp4") {
            return Err("无效的视频文件".to_string());
        }

        // 处理单个文件
        let output_file = output_dir.join("output.mp4");
        merge_videos(
            vec![input_path],
            output_file.clone(),
            duration,
            audio_path.as_ref().to_path_buf(),
            app.clone(),
            &state,
            state.total_files.load(Ordering::SeqCst),
        )?;

        // 返回结果
        Ok(vec![output_file.to_string_lossy().to_string()])
    }
}

// 批处理函数
fn process_batch(
    files: Vec<PathBuf>,
    batch_index: usize,
    duration: u32,
    output_dir: &PathBuf,
    audio_path: &PathBuf,
    state: &ProcessingState,
    app: tauri::AppHandle,
) {
    let group_size = (duration / 4) as usize;
    let total_tasks = (files.len() + group_size - 1) / group_size;  // 计算总任务数
    
    for (group_index, group) in files.chunks(group_size).enumerate() {
        if group.len() == group_size {
            let output_file = generate_output_path(
                output_dir,
                (batch_index * group_size + group_index) as u32
            );

            match merge_videos(
                group.to_vec(),
                output_file.clone(),
                duration,
                audio_path.clone(),
                app.clone(),
                state,
                total_tasks,
            ) {
                Ok(_) => {
                    state.output_files.insert(
                        batch_index * group_size + group_index,
                        output_file.to_string_lossy().to_string()
                    );
                }
                Err(e) => {
                    state.failed_files.insert(
                        output_file.to_string_lossy().to_string(),
                        e
                    );
                }
            }
        }
    }
}

// 创建输出文件夹
fn create_output_dir(input_path: &PathBuf) -> Result<PathBuf, String> {
    let output_dir = input_path.parent().unwrap().join("剪辑后文件");
    // 如果文件夹已存在则先删除
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
    Ok(output_dir)
}
// 生成输出文件路径
fn generate_output_path(output_dir: &PathBuf, index: u32) -> PathBuf {
    output_dir.join(format!("output_{}.mp4", index))
}
// 合并视频并添加音频
fn merge_videos(
    input_files: Vec<PathBuf>,
    output_file: PathBuf,
    duration: u32,
    audio_file: PathBuf,
    app: tauri::AppHandle,
    state: &ProcessingState,
    total_tasks: usize,
) -> Result<(), String> {
    println!("开始合并视频组，输入文件数量: {}", input_files.len());
    app.emit("process-progress", 1).unwrap();

    let mut cmd = Command::new("ffmpeg");
    
    // 检查是否有音频文件
    let has_audio = audio_file.exists() && audio_file.metadata().map(|m| m.len() > 0).unwrap_or(false);

    // 如果只有一个输入文件，使用简单的复制方式
    if input_files.len() == 1 {
        cmd.arg("-i").arg(&input_files[0]);
        
        if has_audio {
            cmd.arg("-i").arg(&audio_file);
            cmd.arg("-c:v").arg("copy")  // 直接复制视频流
               .arg("-c:a").arg("aac")   // 转换音频为 AAC
               .arg("-b:a").arg("128k")  // 音频比特率
               .arg("-shortest");        // 使用最短的流长度
        } else {
            cmd.arg("-c").arg("copy");   // 直接复制所有流
        }

        if duration > 0 {
            cmd.arg("-t").arg(duration.to_string());
        }

        cmd.arg("-y")
           .arg(&output_file);
    } else {
        // 多个文件的合并逻辑保持不变
        let video_info = get_video_dimensions(&input_files[0])?;
        
        for file in &input_files {
            cmd.arg("-i").arg(file);
        }
        
        if has_audio {
            cmd.arg("-i").arg(&audio_file);
        }

        let (width, height) = if video_info.width > video_info.height {
            (1920, 1080)
        } else {
            (1080, 1920)
        };

        let mut filter_complex = String::new();
        for i in 0..input_files.len() {
            filter_complex.push_str(&format!(
                "[{}:v]setpts=PTS-STARTPTS,scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2[v{}];", 
                i, width, height, width, height, i
            ));
        }
        
        for i in 0..input_files.len() {
            filter_complex.push_str(&format!("[v{}]", i));
        }
        filter_complex.push_str(&format!("concat=n={}:v=1:a=0[outv]", input_files.len()));

        cmd.arg("-filter_complex").arg(&filter_complex)
           .arg("-map").arg("[outv]");

        if has_audio {
            cmd.arg("-map").arg(&format!("{}:a", input_files.len()));
        }

        if duration > 0 {
            cmd.arg("-t").arg(duration.to_string());
        }

        cmd.arg("-c:v").arg("libx264")
           .arg("-preset").arg("veryfast")  // 使用更快的预设
           .arg("-crf").arg("26");          // 调整 CRF 值，在速度和质量之间取得平衡

        if has_audio {
            cmd.arg("-c:a").arg("aac")
               .arg("-b:a").arg("128k");
        }

        cmd.arg("-threads").arg("0")
           .arg("-shortest")
           .arg("-y")
           .arg(&output_file);
    }

    println!("执行FFmpeg命令");
    let mut child = cmd.stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    
    // 读取 FFmpeg 的输出并更新进度
    if let Some(stderr) = child.stderr.take() {
        let reader = std::io::BufReader::new(stderr);
        let app_clone = app.clone();
        
        std::thread::spawn(move || {
            use std::io::BufRead;
            for line in reader.lines() {
                if let Ok(line) = line {
                    if line.contains("time=") {
                        // 解析时间信息
                        if let Some(time) = parse_progress(&line) {
                            let progress = ((time as f32 / duration as f32) * 100.0) as u32;
                            app_clone.emit("process-progress", progress).unwrap();
                        }
                    }
                }
            }
        });
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    
    if status.success() {
        // 更新总进度
        let old_progress = state.processed_files.fetch_add(1, Ordering::SeqCst);
        let new_progress = ((old_progress + 1) as f32 / total_tasks as f32 * 100.0) as u32;
        app.emit("process-progress", new_progress).unwrap();
        println!("视频组合并完成: {:?}, 总进度: {}%", output_file, new_progress);
        Ok(())
    } else {
        Err("FFmpeg command failed".to_string())
    }
}

// 添加解析进度的辅助函数
fn parse_progress(line: &str) -> Option<u32> {
    if let Some(time_str) = line.split("time=").nth(1) {
        if let Some(time_part) = time_str.split(' ').next() {
            // 解析时间格式 HH:MM:SS
            let parts: Vec<&str> = time_part.split(':').collect();
            if parts.len() == 3 {
                if let (Ok(h), Ok(m), Ok(s)) = (
                    parts[0].parse::<u32>(),
                    parts[1].parse::<u32>(),
                    parts[2].parse::<f32>()
                ) {
                    return Some(h * 3600 + m * 60 + s as u32);
                }
            }
        }
    }
    None
}

// 新增：获取视频尺寸信息的结构体
#[derive(Debug)]
struct VideoInfo {
    width: i32,
    height: i32,
}

// 新增：获取视频尺寸的函数
fn get_video_dimensions(video_path: &PathBuf) -> Result<VideoInfo, String> {
    println!("执行 ffprobe 命令获取视频尺寸");
    let output = Command::new("ffprobe")
        .arg("-v").arg("quiet")
        .arg("-print_format").arg("json")
        .arg("-show_format")
        .arg("-show_streams")
        .arg(video_path)
        .output()
        .map_err(|e| {
            println!("ffprobe 命令执行失败: {}", e);
            e.to_string()
        })?;

    let output_str = String::from_utf8(output.stdout)
        .map_err(|e| {
            println!("解析 ffprobe 输出失败: {}", e);
            e.to_string()
        })?;
    
    println!("ffprobe 原始输出: {}", output_str);
    
    // 使用 serde_json 解析输出
    let json: serde_json::Value = serde_json::from_str(&output_str)
        .map_err(|e| {
            println!("JSON 解析失败: {}", e);
            e.to_string()
        })?;

    // 尝试从 streams 数组中找到视频流
    if let Some(streams) = json.get("streams") {
        if let Some(streams_array) = streams.as_array() {
            for stream in streams_array {
                if let Some("video") = stream.get("codec_type").and_then(|v| v.as_str()) {
                    let width = stream.get("width")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| "无法获取视频宽度".to_string())? as i32;
                    let height = stream.get("height")
                        .and_then(|v| v.as_i64())
                        .ok_or_else(|| "无法获取视频高度".to_string())? as i32;
                    
                    println!("成功解析视频尺寸 - 宽: {}, 高: {}", width, height);
                    return Ok(VideoInfo { width, height });
                }
            }
        }
    }

    Err("未找到视频流信息".to_string())
}