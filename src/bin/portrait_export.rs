//! Native-only original game-model portrait staging.
fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: portrait_export <new-staging-directory>");
        std::process::exit(1);
    };
    match bevy_open_rts::export_portrait_stage(std::path::Path::new(&path)) {
        Ok(_) => println!(
            "Staged first three full model portraits in {path}; reference/readability review still required"
        ),
        Err(error) => {
            eprintln!("portrait export: {error}");
            std::process::exit(1);
        }
    }
}
