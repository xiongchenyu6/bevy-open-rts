//! Explicit real-system headless skirmish acceptance; writes evidence on timeout.
use bevy_open_rts::{SkirmishAuditConfig, run_skirmish_audit};
use std::{io::Write, path::PathBuf};

fn run() -> Result<bool, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 9 {
        return Err("usage: skirmish_audit <map-id-or-path> <faction-a> <difficulty-a> <spawn-a> <faction-b> <difficulty-b> <spawn-b> <max-seconds> <output.json>".into());
    }
    let parse = |index: usize| {
        args[index]
            .parse::<usize>()
            .map_err(|_| format!("invalid integer '{}'", args[index]))
    };
    let config = SkirmishAuditConfig {
        map: args[0].clone(),
        factions: [args[1].clone(), args[4].clone()],
        difficulties: [args[2].clone(), args[5].clone()],
        spawn_slots: [parse(3)?, parse(6)?],
        max_seconds: args[7].parse().map_err(|_| "invalid simulation limit")?,
    };
    let path = PathBuf::from(&args[8]);
    if path.exists() {
        return Err(format!(
            "refusing to replace existing evidence {}",
            path.display()
        ));
    }
    if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let result = run_skirmish_audit(&config)?;
    let mut evidence = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| error.to_string())?;
    evidence
        .write_all(&serde_json::to_vec_pretty(&result).unwrap())
        .map_err(|error| error.to_string())?;
    println!(
        "{} at {:.3}s; evidence {}",
        result["phase"].as_str().unwrap(),
        result["clock_seconds"].as_f64().unwrap(),
        path.display()
    );
    Ok(result["completed"].as_bool().unwrap())
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => {
            eprintln!("skirmish reached its simulation limit without completion");
            std::process::exit(2);
        }
        Err(error) => {
            eprintln!("skirmish audit: {error}");
            std::process::exit(1);
        }
    }
}
