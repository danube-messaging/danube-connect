//! Delta Lake Sink Connector implementation
//!
//! This connector streams events from Danube topics to Delta Lake tables,
//! supporting S3, Azure Blob Storage, and Google Cloud Storage.

use crate::config::{DeltaLakeSinkConfig, StorageBackend, TopicMapping, WriteMode};
use crate::record::to_record_batch;
use async_trait::async_trait;
use danube_connect_core::{
    ConnectorConfig, ConnectorError, ConnectorResult, ConsumerConfig, SinkConnector, SinkRecord,
    SubscriptionType,
};
use deltalake::operations::create::CreateBuilder;
use deltalake::operations::write::WriteBuilder;
use deltalake::{DeltaOps, DeltaTable, DeltaTableError};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use url::Url;

/// Delta Lake Sink Connector
///
/// Streams events from Danube topics to Delta Lake tables with ACID guarantees.
pub struct DeltaLakeSinkConnector {
    /// Connector configuration
    config: DeltaLakeSinkConfig,

    /// Delta tables cache (table_path -> DeltaTable)
    tables: HashMap<String, Arc<DeltaTable>>,

    /// Record buffers per topic (for batching)
    buffers: HashMap<String, Vec<SinkRecord>>,
}

impl DeltaLakeSinkConnector {
    /// Create a new Delta Lake Sink Connector with configuration
    pub fn with_config(config: DeltaLakeSinkConfig) -> Self {
        Self {
            config,
            tables: HashMap::new(),
            buffers: HashMap::new(),
        }
    }

    /// Get or create a Delta table
    async fn get_or_create_table(
        &mut self,
        mapping: &TopicMapping,
    ) -> ConnectorResult<Arc<DeltaTable>> {
        // Check cache first
        if let Some(table) = self.tables.get(&mapping.delta_table_path) {
            return Ok(Arc::clone(table));
        }

        info!("Opening Delta table at path: {}", mapping.delta_table_path);

        // Configure storage options based on backend
        let storage_options = self.build_storage_options()?;

        // Try to load existing table
        let table_url = Url::parse(&mapping.delta_table_path)
            .map_err(|e| ConnectorError::fatal(format!("Invalid Delta table path URL: {}", e)))?;

        let table = match DeltaOps::try_from_uri_with_storage_options(
            table_url.clone(),
            storage_options.clone(),
        )
        .await
        {
            Ok(ops) => {
                // DeltaOps is a tuple struct wrapping DeltaTable
                let mut table = ops.0;
                match table.load().await {
                    Ok(_) => {
                        info!("Loaded existing Delta table: {}", mapping.delta_table_path);
                        table
                    }
                    Err(DeltaTableError::NotATable(_)) => {
                        // Table doesn't exist, create it
                        info!("Creating new Delta table: {}", mapping.delta_table_path);
                        self.create_table(mapping, storage_options).await?
                    }
                    Err(e) => {
                        return Err(ConnectorError::fatal(format!(
                            "Failed to load Delta table: {}",
                            e
                        )))
                    }
                }
            }
            Err(e) => {
                return Err(ConnectorError::fatal(format!(
                    "Failed to connect to Delta table: {}",
                    e
                )))
            }
        };

        // Cache the table
        let table_arc = Arc::new(table);
        self.tables
            .insert(mapping.delta_table_path.clone(), Arc::clone(&table_arc));

        Ok(table_arc)
    }

    /// Create a new Delta table with user-defined schema
    async fn create_table(
        &self,
        mapping: &TopicMapping,
        storage_options: HashMap<String, String>,
    ) -> ConnectorResult<DeltaTable> {
        // Build Arrow schema from config
        let schema = crate::record::build_arrow_schema(mapping)?;

        // Convert Arrow fields to Delta StructFields
        let delta_fields: Vec<deltalake::kernel::StructField> = schema
            .fields()
            .iter()
            .map(|f| {
                deltalake::kernel::StructField::new(
                    f.name().clone(),
                    arrow_to_delta_type(f.data_type()),
                    f.is_nullable(),
                )
            })
            .collect();

        // Create Delta table
        let table = CreateBuilder::new()
            .with_location(&mapping.delta_table_path)
            .with_storage_options(storage_options)
            .with_columns(delta_fields)
            .await
            .map_err(|e| ConnectorError::fatal(format!("Failed to create Delta table: {}", e)))?;

        info!("Created new Delta table: {}", mapping.delta_table_path);
        Ok(table)
    }

    /// Build storage options based on configured backend
    fn build_storage_options(&self) -> ConnectorResult<HashMap<String, String>> {
        let mut options = HashMap::new();

        match self.config.deltalake.storage_backend {
            StorageBackend::S3 => {
                // AWS credentials from environment (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
                if let Some(region) = &self.config.deltalake.s3_region {
                    options.insert("region".to_string(), region.clone());
                }

                // Custom endpoint for MinIO or S3-compatible storage
                if let Some(endpoint) = &self.config.deltalake.s3_endpoint {
                    options.insert("endpoint".to_string(), endpoint.clone());
                }

                // Allow HTTP for local MinIO testing
                if self.config.deltalake.s3_allow_http {
                    options.insert("allow_http".to_string(), "true".to_string());
                }

                info!("Using S3 storage backend");
            }
            StorageBackend::Azure => {
                // Azure credentials from environment (AZURE_STORAGE_ACCOUNT_KEY or AZURE_STORAGE_SAS_TOKEN)
                if let Some(account) = &self.config.deltalake.azure_storage_account {
                    options.insert("account_name".to_string(), account.clone());
                }

                if let Some(container) = &self.config.deltalake.azure_container {
                    options.insert("container_name".to_string(), container.clone());
                }

                info!("Using Azure Blob Storage backend");
            }
            StorageBackend::GCS => {
                // GCP credentials from environment (GOOGLE_APPLICATION_CREDENTIALS)
                if let Some(project_id) = &self.config.deltalake.gcp_project_id {
                    options.insert("project_id".to_string(), project_id.clone());
                }

                info!("Using Google Cloud Storage backend");
            }
        }

        Ok(options)
    }

    /// Write a batch of records to Delta Lake
    async fn write_batch(
        &mut self,
        mapping: &TopicMapping,
        records: Vec<SinkRecord>,
    ) -> ConnectorResult<()> {
        if records.is_empty() {
            return Ok(());
        }

        debug!(
            "Writing batch of {} records to Delta table: {}",
            records.len(),
            mapping.delta_table_path
        );

        // Convert records to Arrow RecordBatch
        let record_batch = to_record_batch(&records, mapping)?;

        // Get or create Delta table
        let table = self.get_or_create_table(mapping).await?;

        // Write to Delta Lake
        let write_mode = match mapping.write_mode {
            WriteMode::Append => deltalake::protocol::SaveMode::Append,
            WriteMode::Overwrite => deltalake::protocol::SaveMode::Overwrite,
        };

        // Use table's log store for write operation
        // Pass None for snapshot to let WriteBuilder load the latest state
        let log_store = table.log_store();
        let write_builder = WriteBuilder::new(log_store, None)
            .with_input_batches(vec![record_batch])
            .with_save_mode(write_mode);

        // Execute write operation
        let _version = write_builder.await.map_err(|e| {
            ConnectorError::retryable_with_source(
                format!(
                    "Failed to write to Delta table: {}",
                    mapping.delta_table_path
                ),
                e,
            )
        })?;

        info!(
            "Successfully wrote {} records to Delta table: {}",
            records.len(),
            mapping.delta_table_path
        );

        Ok(())
    }

    /// Flush buffered records for a specific topic
    async fn flush_topic(&mut self, topic: &str) -> ConnectorResult<()> {
        if let Some(records) = self.buffers.remove(topic) {
            if records.is_empty() {
                return Ok(());
            }

            // Find the mapping for this topic (clone to avoid borrow issues)
            let mapping = self
                .config
                .deltalake
                .topic_mappings
                .iter()
                .find(|m| m.topic == topic)
                .cloned()
                .ok_or_else(|| {
                    ConnectorError::fatal(format!("No mapping found for topic: {}", topic))
                })?;

            self.write_batch(&mapping, records).await?;
        }

        Ok(())
    }

    /// Flush all buffered records
    async fn flush_all(&mut self) -> ConnectorResult<()> {
        let topics: Vec<String> = self.buffers.keys().cloned().collect();

        for topic in topics {
            if let Err(e) = self.flush_topic(&topic).await {
                error!("Failed to flush topic {}: {}", topic, e);
                return Err(e);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl SinkConnector for DeltaLakeSinkConnector {
    async fn initialize(&mut self, _config: ConnectorConfig) -> ConnectorResult<()> {
        info!("Initializing Delta Lake Sink Connector");
        info!(
            "Connector: {}, Storage Backend: {:?}",
            self.config.core.connector_name, self.config.deltalake.storage_backend
        );

        // Log topic mappings
        for mapping in &self.config.deltalake.topic_mappings {
            info!(
                "Topic Mapping: {} -> {} (schema fields: {})",
                mapping.topic,
                mapping.delta_table_path,
                mapping.schema.len()
            );
        }

        // Initialize buffers
        for mapping in &self.config.deltalake.topic_mappings {
            self.buffers.insert(mapping.topic.clone(), Vec::new());
        }

        info!("Delta Lake Sink Connector initialized successfully");
        Ok(())
    }

    async fn consumer_configs(&self) -> ConnectorResult<Vec<ConsumerConfig>> {
        let configs = self
            .config
            .deltalake
            .topic_mappings
            .iter()
            .map(|mapping| ConsumerConfig {
                topic: mapping.topic.clone(),
                subscription: mapping.subscription.clone(),
                consumer_name: format!(
                    "{}-{}",
                    self.config.core.connector_name, mapping.subscription
                ),
                subscription_type: SubscriptionType::Shared,
            })
            .collect();

        Ok(configs)
    }

    async fn process(&mut self, record: SinkRecord) -> ConnectorResult<()> {
        // Add to buffer
        let topic = record.topic().to_string();
        let buffer = self.buffers.entry(topic.clone()).or_insert_with(Vec::new);
        buffer.push(record);

        // Check if we should flush
        let mapping = self
            .config
            .deltalake
            .topic_mappings
            .iter()
            .find(|m| m.topic == topic)
            .ok_or_else(|| {
                ConnectorError::fatal(format!("No mapping found for topic: {}", topic))
            })?;

        let batch_size = mapping.effective_batch_size(self.config.deltalake.batch_size);

        if buffer.len() >= batch_size {
            debug!(
                "Batch size reached for topic {}, flushing {} records",
                topic,
                buffer.len()
            );
            self.flush_topic(&topic).await?;
        }

        Ok(())
    }

    async fn process_batch(&mut self, records: Vec<SinkRecord>) -> ConnectorResult<()> {
        if records.is_empty() {
            return Ok(());
        }

        debug!("Received {} records to write", records.len());

        // Group records by topic
        let mut by_topic: HashMap<String, Vec<SinkRecord>> = HashMap::new();
        for record in records {
            let topic = record.topic().to_string();
            by_topic.entry(topic).or_insert_with(Vec::new).push(record);
        }

        // Add to buffers and flush if batch size reached
        for (topic, topic_records) in by_topic {
            let buffer = self.buffers.entry(topic.clone()).or_insert_with(Vec::new);
            buffer.extend(topic_records);

            let mapping = self
                .config
                .deltalake
                .topic_mappings
                .iter()
                .find(|m| m.topic == topic)
                .ok_or_else(|| {
                    ConnectorError::fatal(format!("No mapping found for topic: {}", topic))
                })?;

            let batch_size = mapping.effective_batch_size(self.config.deltalake.batch_size);

            // Flush if batch size reached
            if buffer.len() >= batch_size {
                debug!(
                    "Batch size reached for topic {}, flushing {} records",
                    topic,
                    buffer.len()
                );
                self.flush_topic(&topic).await?;
            }
        }

        Ok(())
    }

    async fn shutdown(&mut self) -> ConnectorResult<()> {
        info!("Shutting down Delta Lake Sink Connector");

        // Flush any remaining buffered records
        if let Err(e) = self.flush_all().await {
            warn!("Error flushing records during shutdown: {}", e);
        }

        info!("Delta Lake Sink Connector shutdown complete");
        Ok(())
    }
}

/// Convert Arrow DataType to Delta DataType
fn arrow_to_delta_type(arrow_type: &arrow::datatypes::DataType) -> deltalake::kernel::DataType {
    use arrow::datatypes::DataType as ArrowType;
    use arrow::datatypes::TimeUnit;
    use deltalake::kernel::DataType as DeltaType;
    use deltalake::kernel::PrimitiveType;

    match arrow_type {
        ArrowType::Utf8 => DeltaType::Primitive(PrimitiveType::String),
        ArrowType::Int8 => DeltaType::Primitive(PrimitiveType::Byte),
        ArrowType::Int16 => DeltaType::Primitive(PrimitiveType::Short),
        ArrowType::Int32 => DeltaType::Primitive(PrimitiveType::Integer),
        ArrowType::Int64 => DeltaType::Primitive(PrimitiveType::Long),
        ArrowType::Float32 => DeltaType::Primitive(PrimitiveType::Float),
        ArrowType::Float64 => DeltaType::Primitive(PrimitiveType::Double),
        ArrowType::Boolean => DeltaType::Primitive(PrimitiveType::Boolean),
        ArrowType::Binary => DeltaType::Primitive(PrimitiveType::Binary),
        ArrowType::Timestamp(TimeUnit::Microsecond, _) => {
            DeltaType::Primitive(PrimitiveType::Timestamp)
        }
        ArrowType::Date32 => DeltaType::Primitive(PrimitiveType::Date),
        _ => DeltaType::Primitive(PrimitiveType::String), // Fallback
    }
}
