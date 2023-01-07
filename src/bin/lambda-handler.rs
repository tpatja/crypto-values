use crypto_values::get_updated_values;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde_json::Value;

pub(crate) async fn handler(_event: LambdaEvent<Value>) -> Result<(), Error> {
    match get_updated_values(false, true).await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{}", e);
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let handler = service_fn(handler);
    lambda_runtime::run(handler).await.unwrap();
    Ok(())
}
