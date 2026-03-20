use crate::drawers::image::load_canvas_from_file;
use crate::filesystem::{list_browser_entries, load_virtual_image_bytes, resolve_start_path};
use crate::options::NavigationSortOption;
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct BenchResult {
    pub name: &'static str,
    pub iterations: usize,
    pub total: Duration,
    pub average: Duration,
}

pub fn benchmark_decode(path: &Path, iterations: usize) -> Result<BenchResult, String> {
    let iterations = iterations.max(1);
    let started = Instant::now();
    for _ in 0..iterations {
        load_canvas_from_file(path).map_err(|err| err.to_string())?;
    }
    let total = started.elapsed();
    Ok(BenchResult {
        name: "decode",
        iterations,
        average: total / iterations as u32,
        total,
    })
}

pub fn benchmark_browser_scan(
    path: &Path,
    iterations: usize,
    sort: NavigationSortOption,
) -> Result<BenchResult, String> {
    let iterations = iterations.max(1);
    let started = Instant::now();
    for _ in 0..iterations {
        let _entries = list_browser_entries(path, sort);
    }
    let total = started.elapsed();
    Ok(BenchResult {
        name: "browser-scan",
        iterations,
        average: total / iterations as u32,
        total,
    })
}

pub fn benchmark_archive_read(path: &Path, iterations: usize) -> Result<BenchResult, String> {
    let iterations = iterations.max(1);
    let entries = list_browser_entries(path, NavigationSortOption::OsName);
    let Some(first_entry) = entries.first() else {
        return Err("no readable archive entries".to_string());
    };

    let started = Instant::now();
    for _ in 0..iterations {
        let load_path = resolve_start_path(first_entry)
            .ok_or_else(|| "failed to resolve start path".to_string())?;
        if load_virtual_image_bytes(first_entry).is_none() && !load_path.exists() {
            return Err("failed to read archive entry".to_string());
        }
    }
    let total = started.elapsed();
    Ok(BenchResult {
        name: "archive-read",
        iterations,
        average: total / iterations as u32,
        total,
    })
}
