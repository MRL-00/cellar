mod changes;
mod metadata;
mod profiles;
mod safety;
mod service;

use std::io::{self, Read};

#[tokio::main]
async fn main() {
    let mut input = String::new();
    let response = if io::stdin().take(65_537).read_to_string(&mut input).is_err()
        || input.len() > 65_536
    {
        serde_json::json!({"error": "Request exceeds the input limit"})
    } else {
        match serde_json::from_str::<service::Request>(&input) {
            Ok(request) => {
                match tokio::time::timeout(std::time::Duration::from_secs(5), service::run(request))
                    .await
                {
                    Ok(Ok(value)) => value,
                    Ok(Err(error)) => serde_json::json!({"error": error}),
                    Err(_) => serde_json::json!({"error": "Query timed out after 5 seconds"}),
                }
            }
            Err(_) => serde_json::json!({"error": "Invalid request"}),
        }
    };
    println!("{response}");
}
