use std::path::PathBuf;

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
