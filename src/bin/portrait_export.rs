//! Native-only original game-model portrait staging.
fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(inventory), Some(path)) = (args.next(), args.next()) else {
        eprintln!("usage: portrait_export <inventory.json> <new-staging-directory>");
        std::process::exit(1);
    };
    match bevy_open_rts::export_portrait_stage(
        std::path::Path::new(&inventory),
        std::path::Path::new(&path),
    ) {
        Ok(_) => println!(
            "Staged inventory model portraits in {path}; reference/readability review still required"
        ),
        Err(error) => {
            eprintln!("portrait export: {error}");
            std::process::exit(1);
        }
    }
}
