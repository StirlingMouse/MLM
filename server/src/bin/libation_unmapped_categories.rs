use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use mlm_core::linker::{
    folder::Libation,
    libation_cats::{MappingDepth, three_plus_override_candidates},
};

#[derive(Debug, Clone)]
struct CandidateStat {
    count: usize,
    depth: MappingDepth,
}

fn main() -> Result<()> {
    let root = env::args_os().nth(1).map(PathBuf::from).unwrap_or_default();
    if root.as_os_str().is_empty() {
        bail!(
            "usage: cargo run -p mlm --bin libation_unmapped_categories -- <libation-export-dir>"
        );
    }

    let mut stats: BTreeMap<Vec<String>, CandidateStat> = BTreeMap::new();
    let mut json_files = Vec::new();
    collect_json_files(&root, &mut json_files)?;

    for path in json_files {
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let Ok(meta) = serde_json::from_str::<Libation>(&raw) else {
            continue;
        };

        for candidate in three_plus_override_candidates(&meta.category_ladders) {
            let stat = stats
                .entry(candidate.original_path.clone())
                .or_insert(CandidateStat {
                    count: 0,
                    depth: candidate.depth,
                });
            stat.count += 1;
        }
    }

    let mut rows: Vec<_> = stats.into_iter().collect();
    rows.sort_by(|(left_path, left), (right_path, right)| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.depth.cmp(&right.depth))
            .then_with(|| left_path.cmp(right_path))
    });

    for (path, stat) in rows {
        println!(
            "{:>6}  {:<16}  {}",
            stat.count,
            format!("{:?}", stat.depth),
            path.join(" > ")
        );
    }

    Ok(())
}

fn collect_json_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "json") {
            out.push(path);
        }
    }

    Ok(())
}
