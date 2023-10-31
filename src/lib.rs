pub mod collector;
pub use collector::*;

mod tags;
pub use tags::*;

mod ctracing;
pub use ctracing::TracingCrateFilter;

mod storage;
pub use storage::*;

mod excelstats;
pub use excelstats::ExcelStats;

pub fn parse_storage(args: &str) -> Result<Box<dyn StorageBackend>, &'static str> {
    args.split("->")
        .filter_map(|arg| match arg {
            "file" => {
                let store_path =
                    std::env::var("STORE_PATH").unwrap_or_else(|_| "data.json".to_string());

                Some(Box::new(storage::FileStorage::new(store_path)) as Box<dyn StorageBackend>)
            }
            "s3" => {
                let s3_bucket = std::env::var("S3_BUCKET").expect("Missing `S3_BUCKET`");
                let s3_access_key =
                    std::env::var("S3_ACCESS_KEY").expect("Missing `S3_ACCESS_KEY`");
                let s3_secret_key =
                    std::env::var("S3_SECRET_KEY").expect("Missing `S3_SECRET_KEY`");
                let s3_endpoint = std::env::var("S3_ENDPOINT").expect("Missing `S3_ENDPOINT`");

                Some(Box::new(S3Storage::new(
                    s3::Bucket::new(
                        &s3_bucket,
                        s3::Region::Custom {
                            region: "default".to_string(),
                            endpoint: s3_endpoint.to_string(),
                        },
                        s3::creds::Credentials::new(
                            Some(&s3_access_key),
                            Some(&s3_secret_key),
                            None,
                            None,
                            None,
                        )
                        .unwrap(),
                    )
                    .unwrap()
                    .with_path_style(),
                )))
            }
            other => {
                tracing::error!("Unknown Storage {:?}", other);

                None
            }
        })
        .reduce(
            |acc: Box<dyn StorageBackend>, elem: Box<dyn StorageBackend>| {
                Box::new(storage::Replicated::new(acc, elem)) as Box<dyn StorageBackend>
            },
        )
        .ok_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single() {
        std::env::set_var("S3", "");
        parse_storage("s3").unwrap();
    }
}
