use std::{path::PathBuf, pin::Pin};

use crate::StorageBackend;

pub struct FileStorage {
    path: PathBuf,
}

impl FileStorage {
    pub fn new<P>(path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        Self { path: path.into() }
    }

    pub async fn read(&mut self) -> Result<Vec<u8>, std::io::Error> {
        tokio::fs::read(&self.path).await
    }

    pub async fn write(&mut self, content: &[u8]) -> Result<(), std::io::Error> {
        if !self.path.exists() {
            tokio::fs::File::create(&self.path).await?;
        }

        tokio::fs::write(&self.path, content).await
    }
}

impl StorageBackend for FileStorage {
    fn write(
        &mut self,
        content: Vec<u8>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(), ()>> + Send + 'static>> {
        let path = self.path.clone();

        Box::pin(async move {
            if !path.exists() {
                tokio::fs::File::create(&path).await.map_err(|e| {
                    tracing::error!("Creating file {:?}", e);
                    ()
                })?;
            }

            tokio::fs::write(&path, &content).await.map_err(|e| {
                tracing::error!("Writing File {:?}", e);
                ()
            })
        })
    }

    fn load(
        &mut self,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, ()>> + Send + 'static>> {
        let path = self.path.clone();

        Box::pin(async move { tokio::fs::read(&path).await.map_err(|e| ()) })
    }
}
