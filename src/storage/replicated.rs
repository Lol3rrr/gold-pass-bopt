use crate::StorageBackend;

pub struct Replicated {
    primary: Box<dyn StorageBackend>,
    secondary: Box<dyn StorageBackend>,
}

impl Replicated {
    pub fn new(primary: Box<dyn StorageBackend>, secondary: Box<dyn StorageBackend>) -> Self {
        Self { primary, secondary }
    }
}

impl StorageBackend for Replicated {
    #[tracing::instrument(skip(self, content))]
    fn write(
        &mut self,
        content: Vec<u8>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ()>> + Send + 'static>> {
        let pfut = self.primary.write(content.clone());
        let sfut = self.secondary.write(content);

        Box::pin(async move {
            tracing::trace!("Storing Replicated");

            let pres = pfut.await;
            let sres = sfut.await;

            pres.ok().or(sres.ok()).ok_or(())
        })
    }

    fn load(
        &mut self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, ()>> + Send + 'static>>
    {
        let pfut = self.primary.load();
        let sfut = self.secondary.load();

        Box::pin(async move {
            if let Ok(r) = pfut.await {
                return Ok(r);
            }

            sfut.await
        })
    }
}
