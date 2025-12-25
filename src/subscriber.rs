//! RabbitMQ subscription for receiving monitor snapshots.
//!
//! Adapted from buswatch-tui's subscribe module.

use anyhow::{Context, Result};
use buswatch_types::Snapshot;
use config::{Config, Environment, File};
use futures_util::StreamExt;
use lapin::{
    options::{BasicConsumeOptions, QueueBindOptions, QueueDeclareOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use std::path::Path;
use tokio::sync::mpsc;

/// Create a subscriber that connects to RabbitMQ and streams snapshots.
///
/// # Arguments
///
/// * `config_path` - Path to config file with RabbitMQ settings
/// * `topic` - The topic pattern to subscribe to
///
/// # Returns
///
/// A receiver channel that yields snapshots as they arrive.
pub async fn create_subscriber(
    config_path: &Path,
    topic: &str,
) -> Result<(
    mpsc::UnboundedReceiver<Snapshot>,
    tokio::task::JoinHandle<()>,
)> {
    // Load config
    let config = Config::builder()
        .add_source(File::from(config_path).required(false))
        .add_source(Environment::with_prefix("CARYATID"))
        .build()?;

    // Extract RabbitMQ config
    let (url, exchange) = extract_rabbitmq_config(&config)?;

    tracing::info!("Connecting to RabbitMQ at {}", url);

    // Connect to RabbitMQ
    let conn = Connection::connect(&url, ConnectionProperties::default())
        .await
        .context("Failed to connect to RabbitMQ")?;

    let channel = conn.create_channel().await?;

    // Try to consume from an existing named queue first (caryatid creates these),
    // otherwise create a temporary exclusive queue bound to the exchange
    let queue_name = if channel
        .queue_declare(
            topic,
            QueueDeclareOptions {
                passive: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .is_ok()
    {
        tracing::info!("Using existing queue: {}", topic);
        topic.to_string()
    } else {
        tracing::info!("Creating temporary queue bound to exchange: {}", exchange);
        let queue = channel
            .queue_declare(
                "",
                QueueDeclareOptions {
                    exclusive: true,
                    auto_delete: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;

        channel
            .queue_bind(
                queue.name().as_str(),
                &exchange,
                topic,
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;

        queue.name().to_string()
    };

    // Start consuming
    let mut consumer = channel
        .basic_consume(
            &queue_name,
            "neuronic",
            BasicConsumeOptions {
                no_ack: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    // Create channel for forwarding to UI
    let (tx, rx) = mpsc::unbounded_channel();

    // Spawn background task to read messages
    let handle = tokio::spawn(async move {
        tracing::info!("Started consuming from queue");
        while let Some(delivery) = consumer.next().await {
            match delivery {
                Ok(delivery) => match minicbor_serde::from_slice::<Snapshot>(&delivery.data) {
                    Ok(snapshot) => {
                        if tx.send(snapshot).is_err() {
                            tracing::info!("Receiver dropped, stopping consumer");
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to deserialize snapshot: {}", e);
                    }
                },
                Err(e) => {
                    tracing::error!("Consumer error: {}", e);
                    break;
                }
            }
        }
        tracing::info!("Consumer task finished");
    });

    Ok((rx, handle))
}

/// Extract RabbitMQ URL and exchange from config.
fn extract_rabbitmq_config(config: &Config) -> Result<(String, String)> {
    // Try [rabbitmq] format first
    if let Ok(rabbitmq) = config.get_table("rabbitmq") {
        let url = rabbitmq
            .get("url")
            .and_then(|v| v.clone().into_string().ok())
            .ok_or_else(|| anyhow::anyhow!("Missing 'url' in [rabbitmq]"))?;
        let exchange = rabbitmq
            .get("exchange")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_else(|| "caryatid".to_string());
        return Ok((url, exchange));
    }

    // Try [message-bus.*] format (caryatid style)
    if let Ok(message_bus) = config.get_table("message-bus") {
        for (_id, bus_conf) in message_bus {
            if let Ok(tbl) = bus_conf.into_table() {
                let class = tbl.get("class").and_then(|v| v.clone().into_string().ok());
                if class.as_deref() == Some("rabbit-mq") {
                    let url = tbl
                        .get("url")
                        .and_then(|v| v.clone().into_string().ok())
                        .ok_or_else(|| anyhow::anyhow!("Missing 'url' in rabbit-mq bus config"))?;
                    let exchange = tbl
                        .get("exchange")
                        .and_then(|v| v.clone().into_string().ok())
                        .unwrap_or_else(|| "caryatid".to_string());
                    return Ok((url, exchange));
                }
            }
        }
    }

    // Default fallback for development
    tracing::warn!("No RabbitMQ config found, using default localhost");
    Ok((
        "amqp://127.0.0.1:5672/%2f".to_string(),
        "caryatid".to_string(),
    ))
}
