use fetch_data;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    fetch_data::fetch_data().await
}