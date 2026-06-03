//! clean 子命令 — 清理缓存目录 `~/.mcu_bridge/`。
//!
//! 设计文档 §8：正常退出自动清理，异常退出保留。
//! `mcu-bridge clean` 清理当前项目，`--all` 清理所有，`--older-than 7d` 按时间清理。

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// clean 子命令参数
pub struct CleanArgs {
    pub all: bool,
    pub older_than: Option<String>,
}

/// 计算当前项目（cwd）的 hash，作为缓存子目录名。
fn project_hash() -> String {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut hasher = DefaultHasher::new();
    cwd.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// 解析 `--older-than` 时间字符串，返回 Duration。
///
/// 格式: `<N>s|m|h|d|w`
fn parse_duration(s: &str) -> anyhow::Result<Duration> {
    if s.len() < 2 {
        anyhow::bail!("invalid duration: '{}'. Use format like 7d, 24h, 30m", s);
    }
    let (num_str, suffix) = s.split_at(s.len() - 1);
    let num: u64 = num_str
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid number in duration: '{}'", s))?;
    let secs = match suffix {
        "s" => num,
        "m" => num * 60,
        "h" => num * 3600,
        "d" => num * 86400,
        "w" => num * 604800,
        _ => anyhow::bail!("unknown duration suffix '{}'. Use s/m/h/d/w", suffix),
    };
    Ok(Duration::from_secs(secs))
}

/// 获取缓存根目录路径。
fn cache_root() -> anyhow::Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    Ok(home.join(".mcu_bridge"))
}

/// 处理 clean 子命令
pub fn handle(args: &CleanArgs) -> anyhow::Result<()> {
    let root = cache_root()?;
    if !root.exists() || !root.is_dir() {
        println!("nothing to clean (cache directory does not exist)");
        return Ok(());
    }

    if args.all {
        let count = count_dirs(&root)?;
        fs::remove_dir_all(&root)?;
        println!("removed entire cache directory ({} session(s))", count);
        return Ok(());
    }

    if let Some(ref older_than) = args.older_than {
        let threshold = parse_duration(older_than)?;
        let now = SystemTime::now();
        let mut removed = 0usize;
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let meta = entry.metadata()?;
                if let Ok(modified) = meta.modified() {
                    if let Ok(elapsed) = now.duration_since(modified) {
                        if elapsed > threshold {
                            fs::remove_dir_all(entry.path())?;
                            removed += 1;
                        }
                    }
                }
            }
        }
        println!("removed {} session(s) older than {}", removed, older_than);
        return Ok(());
    }

    // 默认：清理当前项目
    let hash = project_hash();
    let proj_dir = root.join(&hash);
    if proj_dir.exists() {
        let count = count_dirs(&proj_dir)?;
        fs::remove_dir_all(&proj_dir)?;
        println!("removed {} session(s) for current project", count);
    } else {
        println!("nothing to clean (project cache not found)");
    }
    Ok(())
}

/// 递归计算目录中所有子目录的数量。
fn count_dirs(path: &std::path::Path) -> std::io::Result<usize> {
    let mut count = 0;
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                // 递归计数子目录
                count += 1 + count_dirs(&entry.path())?;
            }
        }
    }
    Ok(count)
}
