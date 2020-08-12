use sqlx::postgres::{PgPoolOptions, PgPool};
use sqlx::Result;
use rocket::State;
use airq;

#[rocket::get("/")]
async fn hello() -> &'static str {
    "uiae"
}

#[rocket::launch]
async fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", rocket::routes![hello])
}
