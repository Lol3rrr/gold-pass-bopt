pub struct TracingCrateFilter {}

impl<S> tracing_subscriber::Layer<S> for TracingCrateFilter
where
    S: tracing::Subscriber,
{
    fn enabled(
        &self,
        metadata: &tracing::Metadata<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        metadata.target().contains("gold_pass_bot")
    }
}
