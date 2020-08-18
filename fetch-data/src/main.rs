use fetch_data::{Error, FetchData};

#[tokio::main]
async fn main() -> Result<(), Error> {
    FetchData::new().fetch_data().await
}