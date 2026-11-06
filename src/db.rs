//! The database connection.

use toasty::Db;
use topcoat::context::{Cx, app_context};

/// The local SQLite database file, created when it is first opened. `DATABASE_URL`
/// moves it, such as onto a persistent volume in a container.
const DEFAULT_DATABASE_URL: &str = "sqlite:data.db";

/// Opens the database with every model in the application.
///
/// The tables come from migrations. After changing a model, generate a migration with
/// `cargo run -- toasty migration generate` and apply it with
/// `cargo run -- toasty migration apply`.
pub async fn connect() -> toasty::Result<Db> {
    Db::builder()
        .models(toasty::models!(crate::*))
        .connect(&std::env::var("DATABASE_URL").ok().filter(|u| !u.is_empty()).unwrap_or_else(|| DEFAULT_DATABASE_URL.to_owned()))
        .await
}

/// Returns the database handle from the router's app context. Cloning it is cheap
/// because it refers to a shared connection pool.
pub fn db(cx: &Cx) -> Db {
    app_context::<Db>(cx).clone()
}
