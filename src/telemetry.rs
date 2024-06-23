use tracing::Subscriber;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, FmtSubscriber, Registry};
use tracing::subscriber::set_global_default;
use tracing_log::LogTracer;

pub fn get_subscriber<Sink>(
    name: impl Into<String>,
    env_filter: impl Into<String>,
    sink: Sink,
) -> impl Subscriber + Sync + Send
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter.into()));
    // let formatting_layer = BunyanFormattingLayer::new(name.into(), sink);
		let formatting_layer = tracing_subscriber::fmt::Layer::new()
		.with_writer(sink)
		.with_target(false)
		.with_thread_ids(true)
		.with_thread_names(true)
		.with_ansi(true)
		.with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339());
    Registry::default()
        .with(env_filter)
        .with(formatting_layer)
        // .with(JsonStorageLayer)
}

pub fn init_subscriber(subscriber: impl Subscriber + Sync + Send) {
	LogTracer::init().expect("Failed to initialize logger");
	set_global_default(subscriber).expect("Failed to set subscriber");
}
