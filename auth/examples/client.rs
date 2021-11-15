use ate_auth::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MyData {
    pi: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ate::log_init(0, false);

    let dio = DioBuilder::default()
        .with_session_prompt()
        .await?
        .build("mychain")
        .await?;

    dio.store(MyData {
        pi: "3.14159265359".to_string(),
    })?;
    dio.commit().await?;

    Ok(())
}
