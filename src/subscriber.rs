//! RabbitMQ subscription for receiving monitor snapshots.

use anyhow::{Context, Result};
use buswatch_types::Snapshot;
use futures_util::StreamExt;
use lapin::{
    options::{BasicConsumeOptions, QueueBindOptions, QueueDeclareOptions},
    types::FieldTable,
    Connection, ConnectionProperties,
};
use tokio::sync::mpsc;

use crate::config::SubscriberConfig;

/// Create a subscriber that connects to RabbitMQ and streams snapshots.
///
/// # Arguments
///
/// * `config` - Subscriber configuration with connection settings
/// * `topic` - The topic pattern to subscribe to
///
/// # Returns
///
/// A receiver channel that yields snapshots as they arrive.
pub async fn create_subscriber(
    config: &SubscriberConfig,
    topic: &str,
) -> Result<(
    mpsc::UnboundedReceiver<Snapshot>,
    tokio::task::JoinHandle<()>,
)> {
    tracing::info!("Connecting to RabbitMQ at {}", config.url);

    // Connect to RabbitMQ
    let conn = Connection::connect(&config.url, ConnectionProperties::default())
        .await
        .context("Failed to connect to RabbitMQ")?;

    let channel = conn.create_channel().await?;

    // Create a temporary exclusive queue bound to the exchange with the topic pattern
    tracing::info!(
        "Creating temporary queue bound to exchange: {}",
        config.exchange
    );
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
            &config.exchange,
            topic,
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    let queue_name = queue.name().to_string();

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
