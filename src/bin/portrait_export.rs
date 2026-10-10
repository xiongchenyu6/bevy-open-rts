//! Native-only original game-model portrait staging.
fn main() {
    let mut args = std::env::args().skip(1);
    let first = args.next();
    let roster = first.as_deref() == Some("--roster");
    let inventory = if roster { args.next() } else { first };
    let (Some(inventory), Some(path)) = (inventory, args.next()) else {
        eprintln!("usage: portrait_export [--roster] <inventory.json> <new-staging-directory>");
        std::process::exit(1);
    };
    let export = if roster {
        bevy_open_rts::export_roster_stage
    } else {
        bevy_open_rts::export_portrait_stage
    };
    match export(
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
