mod app;
mod components;
mod db;
mod features;

// The interface text, in `src/l10n/{locale}.ftl`. Creates `t!`.
topcoat::l10n::fluent::messages!("./l10n/{locale}.ftl");

#[tokio::main]
async fn main() {
    let db = db::connect().await.unwrap();

    // `cargo run -- toasty migration <command>` manages the database schema.
    let mut args = std::env::args().skip(1).peekable();
    if args.peek().is_some_and(|arg| arg == "toasty") {
        if let Err(error) = toasty_cli::ToastyCli::new(db).parse_from(args).await {
            eprintln!("{error:#}");
            std::process::exit(1);
        }
        return;
    }

    // Every library page reads `metadata.db`, and without it they fail with a bare 500
    // that names no cause, so stop here with one that does.
    let metadata = features::library::root().join("metadata.db");
    if !metadata.is_file() {
        let shown = std::path::absolute(&metadata).unwrap_or(metadata);
        eprintln!(
            "no Calibre library at {}: set CALIBRE_LIBRARY to the folder that contains metadata.db",
            shown.display()
        );
        std::process::exit(1);
    }

    // Creates or updates the tables before serving, so a missing or new `data.db` works.
    if let Err(error) = toasty_cli::ToastyCli::new(db.clone()).parse_from(["toasty", "migration", "apply"]).await {
        eprintln!("applying database migrations failed: {error:#}");
        std::process::exit(1);
    }

    topcoat::start(app::router().app_context(db).build()).await.unwrap();
}
