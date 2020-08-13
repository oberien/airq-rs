use fetch_data::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    fetch_data::fetch_data().await
}