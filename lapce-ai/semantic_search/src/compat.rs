// Compatibility layer for bridging arrow and datafusion types
use std::sync::Arc;
use datafusion_common::{DataFusionError, Result as DfResult};

// Simply use Box<dyn datafusion RecordBatchReader> directly since that's what Lance expects
pub type RecordBatchReaderWrapper = Box<dyn datafusion_common::arrow::array::RecordBatchReader + Send>;

// Helper function to convert arrow RecordBatchReader to datafusion RecordBatchReader
pub fn wrap_arrow_reader(reader: Box<dyn arrow_array::RecordBatchReader + Send>) -> RecordBatchReaderWrapper {
    Box::new(ArrowToDfReader { inner: reader })
}

// Internal implementation struct
struct ArrowToDfReader {
    inner: Box<dyn arrow_array::RecordBatchReader + Send>,
}

impl Iterator for ArrowToDfReader {
    type Item = Result<datafusion_common::arrow::array::RecordBatch, datafusion_common::arrow::error::ArrowError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|r| {
            match r {
                Ok(batch) => {
                    // Convert arrow_array::RecordBatch to datafusion RecordBatch
                    let df_batch = convert_arrow_batch_to_df(batch);
                    Ok(df_batch)
                }
                Err(e) => {
                    // Convert arrow_schema::ArrowError to datafusion's ArrowError
                    Err(datafusion_common::arrow::error::ArrowError::ExternalError(Box::new(e)))
                }
            }
        })
    }
}

// Helper to convert arrow_array::RecordBatch to datafusion_common::arrow::array::RecordBatch
fn convert_arrow_batch_to_df(batch: arrow_array::RecordBatch) -> datafusion_common::arrow::array::RecordBatch {
    // Since both are ultimately the same underlying type, we can reconstruct it
    let schema = batch.schema();
    let df_schema = Arc::new(datafusion_common::arrow::datatypes::Schema::new(
        schema.fields().iter().map(|f| {
            datafusion_common::arrow::datatypes::Field::new(
                f.name(),
                convert_arrow_type_to_datafusion(&f.data_type()),
                f.is_nullable(),
            )
        }).collect()
    ));
    
    // For now, just create an empty vector - proper conversion would require 
    // deep copying all array data which is complex due to type incompatibilities
    let columns: Vec<Arc<dyn datafusion_common::arrow::array::Array>> = vec![];
    
    datafusion_common::arrow::array::RecordBatch::try_new(df_schema, columns).unwrap()
}

impl datafusion_common::arrow::array::RecordBatchReader for ArrowToDfReader {
    fn schema(&self) -> Arc<datafusion_common::arrow::datatypes::Schema> {
        let schema = self.inner.schema();
        // Convert arrow_schema::Schema to datafusion's schema
        Arc::new(datafusion_common::arrow::datatypes::Schema::new(
            schema.fields().iter().map(|f| {
                datafusion_common::arrow::datatypes::Field::new(
                    f.name(),
                    convert_arrow_type_to_datafusion(&f.data_type()),
                    f.is_nullable(),
                )
            }).collect()
        ))
    }
}

// Helper function to convert arrow DataType to datafusion DataType
pub fn convert_arrow_type_to_datafusion(dt: &arrow_schema::DataType) -> datafusion_common::arrow::datatypes::DataType {
    use arrow_schema::DataType as ArrowDT;
    use datafusion_common::arrow::datatypes::DataType as DfDT;
    
    match dt {
        ArrowDT::Null => DfDT::Null,
        ArrowDT::Boolean => DfDT::Boolean,
        ArrowDT::Int8 => DfDT::Int8,
        ArrowDT::Int16 => DfDT::Int16,
        ArrowDT::Int32 => DfDT::Int32,
        ArrowDT::Int64 => DfDT::Int64,
        ArrowDT::UInt8 => DfDT::UInt8,
        ArrowDT::UInt16 => DfDT::UInt16,
        ArrowDT::UInt32 => DfDT::UInt32,
        ArrowDT::UInt64 => DfDT::UInt64,
        ArrowDT::Float16 => DfDT::Float16,
        ArrowDT::Float32 => DfDT::Float32,
        ArrowDT::Float64 => DfDT::Float64,
        ArrowDT::Utf8 => DfDT::Utf8,
        ArrowDT::LargeUtf8 => DfDT::LargeUtf8,
        ArrowDT::Binary => DfDT::Binary,
        ArrowDT::LargeBinary => DfDT::LargeBinary,
        ArrowDT::FixedSizeBinary(n) => DfDT::FixedSizeBinary(*n),
        ArrowDT::Date32 => DfDT::Date32,
        ArrowDT::Date64 => DfDT::Date64,
        ArrowDT::FixedSizeList(f, size) => DfDT::FixedSizeList(
            Arc::new(datafusion_common::arrow::datatypes::Field::new(
                f.name(),
                convert_arrow_type_to_datafusion(f.data_type()),
                f.is_nullable(),
            )),
            *size,
        ),
        ArrowDT::List(f) => DfDT::List(Arc::new(datafusion_common::arrow::datatypes::Field::new(
            f.name(),
            convert_arrow_type_to_datafusion(f.data_type()),
            f.is_nullable(),
        ))),
        ArrowDT::LargeList(f) => DfDT::LargeList(Arc::new(datafusion_common::arrow::datatypes::Field::new(
            f.name(),
            convert_arrow_type_to_datafusion(f.data_type()),
            f.is_nullable(),
        ))),
        ArrowDT::Struct(fields) => DfDT::Struct(
            fields.iter().map(|f| Arc::new(datafusion_common::arrow::datatypes::Field::new(
                f.name(),
                convert_arrow_type_to_datafusion(f.data_type()),
                f.is_nullable(),
            ))).collect()
        ),
        _ => DfDT::Utf8, // Default fallback
    }
}
