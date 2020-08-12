use sqlx::postgres::{PgPoolOptions, PgPool};
use sqlx::Result;
use rocket::State;
use airq;

#[rocket::get("/")]
async fn hello() -> &'static str {
    "uiae"
}

fn create_pool() -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(8)
        .connect_lazy(env!("DATABASE_URL"))
}

#[rocket::launch]
async fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .manage(create_pool())
        .mount("/", rocket::routes![hello])
}
