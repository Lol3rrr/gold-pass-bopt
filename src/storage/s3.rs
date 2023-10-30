use crate::StorageBackend;

pub struct S3Storage {
    bucket: s3::Bucket,
    filename: String,
}

impl S3Storage {
    pub fn new(bucket: s3::Bucket) -> Self {
        Self {
            bucket,
            filename: "storage.json".to_string(),
        }
    }
}

impl StorageBackend for S3Storage {
    #[tracing::instrument(skip(self, content))]
    fn write(
        &mut self,
        content: Vec<u8>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ()>> + Send + 'static>> {
        let bucket = self.bucket.clone();
        let filename = self.filename.clone();

        Box::pin(async move {
            let filename = filename;
            let content = content;

            tracing::trace!("Storing to S3 Bucket");

            let res = bucket.put_object(&filename, &content);

            match res.await {
                Ok(_) => Ok(()),
                Err(e) => {
                    tracing::error!("{:?}", e);

                    Err(())
                }
            }
        })
    }

    fn load(
        &mut self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, ()>> + Send + 'static>>
    {
        let bucket = self.bucket.clone();
        let filename = self.filename.clone();

        Box::pin(async move {
            match bucket.get_object(filename).await {
                Ok(c) => Ok(c.to_vec()),
                Err(e) => Err(()),
            }
        })
    }
}
