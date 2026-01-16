//! Tuple value extraction helpers for direct heap operations.
//!
//! When reading tuples from the heap, we need to extract individual column
//! values. This module provides type-safe extractors for common PostgreSQL
//! types used in CALIBER entities.
//!
//! # Supported Types
//!
//! - UUID (EntityId)
//! - TEXT (String)
//! - INT4 (i32)
//! - INT8 (i64)
//! - BOOL (bool)
//! - TIMESTAMPTZ (Timestamp)
//! - JSONB (serde_json::Value)
//! - BYTEA (Vec<u8>)
//! - FLOAT4 ARRAY (Vec<f32> for embeddings)
//!
//! # Usage
//!
//! ```ignore
//! use crate::tuple_extract::*;
//!
//! // Extract a UUID from column 1
//! let id: Option<uuid::Uuid> = extract_uuid(tuple, tuple_desc, 1)?;
//!
//! // Extract text from column 2
//! let name: Option<String> = extract_text(tuple, tuple_desc, 2)?;
//!
//! // Extract all values as datums
//! let values = extract_all_datums(tuple, tuple_desc)?;
//!
//! // Extract null flags
//! let nulls = extract_all_nulls(tuple, tuple_desc)?;
//! ```

use pgrx::prelude::*;
use pgrx::pg_sys;
use pgrx::datum::TimestampWithTimeZone;

use caliber_core::{CaliberError, CaliberResult, StorageError};
use chrono::{Datelike, Timelike};

/// Extract a single datum value from a heap tuple at the specified attribute number.
///
/// # Arguments
/// * `tuple` - The heap tuple to extract from
/// * `tuple_desc` - The tuple descriptor for the relation
/// * `attnum` - The attribute number (1-based column index)
///
/// # Returns
/// * `Ok((datum, is_null))` - The datum value and null flag
/// * `Err(CaliberError)` - If extraction fails
///
/// # Safety
/// The tuple and tuple_desc must be valid and correspond to each other.
pub unsafe fn extract_datum(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
    attnum: i16,
) -> CaliberResult<(pg_sys::Datum, bool)> {
    if tuple.is_null() {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: "Cannot extract datum from null tuple".to_string(),
        }));
    }

    if tuple_desc.is_null() {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: "Cannot extract datum with null tuple descriptor".to_string(),
        }));
    }

    // Validate attribute number
    let natts = (*tuple_desc).natts as i16;
    if attnum < 1 || attnum > natts {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!(
                "Invalid attribute number {}: must be between 1 and {}",
                attnum, natts
            ),
        }));
    }

    let mut is_null: bool = false;

    let datum = pg_sys::heap_getattr(
        tuple,
        attnum as i32,
        tuple_desc,
        &mut is_null,
    );

    Ok((datum, is_null))
}

/// Extract all datum values from a heap tuple.
///
/// # Arguments
/// * `tuple` - The heap tuple to extract from
/// * `tuple_desc` - The tuple descriptor for the relation
///
/// # Returns
/// * `Ok(Vec<Datum>)` - Vector of datum values, one per column
/// * `Err(CaliberError)` - If extraction fails
///
/// # Note
/// For null values, the datum will be 0 (null datum). Use `extract_all_nulls`
/// to get the null flags.
/// # Safety
/// The tuple and tuple_desc must be valid and correspond to each other.
pub unsafe fn extract_all_datums(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
) -> CaliberResult<Vec<pg_sys::Datum>> {
    if tuple.is_null() || tuple_desc.is_null() {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: "Cannot extract datums from null tuple or descriptor".to_string(),
        }));
    }

    let natts = (*tuple_desc).natts as usize;
    let mut values = Vec::with_capacity(natts);

    for i in 1..=natts {
        let (datum, _is_null) = unsafe { extract_datum(tuple, tuple_desc, i as i16) }?;
        values.push(datum);
    }

    Ok(values)
}

/// Extract all null flags from a heap tuple.
///
/// # Arguments
/// * `tuple` - The heap tuple to extract from
/// * `tuple_desc` - The tuple descriptor for the relation
///
/// # Returns
/// * `Ok(Vec<bool>)` - Vector of null flags, one per column (true = NULL)
/// * `Err(CaliberError)` - If extraction fails
/// # Safety
/// The tuple and tuple_desc must be valid and correspond to each other.
pub unsafe fn extract_all_nulls(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
) -> CaliberResult<Vec<bool>> {
    if tuple.is_null() || tuple_desc.is_null() {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: "Cannot extract nulls from null tuple or descriptor".to_string(),
        }));
    }

    let natts = (*tuple_desc).natts as usize;
    let mut nulls = Vec::with_capacity(natts);

    for i in 1..=natts {
        let (_datum, is_null) = unsafe { extract_datum(tuple, tuple_desc, i as i16) }?;
        nulls.push(is_null);
    }

    Ok(nulls)
}


/// Extract both datum values and null flags from a heap tuple.
///
/// This is more efficient than calling `extract_all_datums` and `extract_all_nulls`
/// separately when you need both.
///
/// # Arguments
/// * `tuple` - The heap tuple to extract from
/// * `tuple_desc` - The tuple descriptor for the relation
///
/// # Returns
/// * `Ok((Vec<Datum>, Vec<bool>))` - Tuple of (values, nulls)
/// * `Err(CaliberError)` - If extraction fails
/// # Safety
/// The tuple and tuple_desc must be valid and correspond to each other.
pub unsafe fn extract_values_and_nulls(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
) -> CaliberResult<(Vec<pg_sys::Datum>, Vec<bool>)> {
    if tuple.is_null() || tuple_desc.is_null() {
        return Err(CaliberError::Storage(StorageError::TransactionFailed {
            reason: "Cannot extract from null tuple or descriptor".to_string(),
        }));
    }

    let natts = (*tuple_desc).natts as usize;
    let mut values = Vec::with_capacity(natts);
    let mut nulls = Vec::with_capacity(natts);

    for i in 1..=natts {
        let (datum, is_null) = unsafe { extract_datum(tuple, tuple_desc, i as i16) }?;
        values.push(datum);
        nulls.push(is_null);
    }

    Ok((values, nulls))
}

// ============================================================================
// TYPE-SPECIFIC EXTRACTORS
// ============================================================================

/// Extract a UUID value from a heap tuple.
///
/// # Arguments
/// * `tuple` - The heap tuple to extract from
/// * `tuple_desc` - The tuple descriptor for the relation
/// * `attnum` - The attribute number (1-based column index)
///
/// # Returns
/// * `Ok(Some(Uuid))` - The UUID value if not null
/// * `Ok(None)` - If the value is null
/// * `Err(CaliberError)` - If extraction fails
pub unsafe fn extract_uuid(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
    attnum: i16,
) -> CaliberResult<Option<uuid::Uuid>> {
    let (datum, is_null) = unsafe { extract_datum(tuple, tuple_desc, attnum) }?;

    if is_null {
        return Ok(None);
    }

    // Convert datum to pgrx::Uuid, then to uuid::Uuid
    let pg_uuid: pgrx::Uuid = unsafe { pgrx::Uuid::from_datum(datum, false) }
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!("Failed to convert datum to UUID at column {}", attnum),
        }))?;

    // Convert pgrx::Uuid bytes to uuid::Uuid
    let uuid = uuid::Uuid::from_bytes(*pg_uuid.as_bytes());
    Ok(Some(uuid))
}

/// Extract a text/varchar value from a heap tuple.
///
/// # Arguments
/// * `tuple` - The heap tuple to extract from
/// * `tuple_desc` - The tuple descriptor for the relation
/// * `attnum` - The attribute number (1-based column index)
///
/// # Returns
/// * `Ok(Some(String))` - The text value if not null
/// * `Ok(None)` - If the value is null
/// * `Err(CaliberError)` - If extraction fails
pub unsafe fn extract_text(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
    attnum: i16,
) -> CaliberResult<Option<String>> {
    let (datum, is_null) = unsafe { extract_datum(tuple, tuple_desc, attnum) }?;

    if is_null {
        return Ok(None);
    }

    // Convert datum to String
    let text: String = unsafe { String::from_datum(datum, false) }
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!("Failed to convert datum to text at column {}", attnum),
        }))?;

    Ok(Some(text))
}


/// Extract an i32 (INT4) value from a heap tuple.
///
/// # Arguments
/// * `tuple` - The heap tuple to extract from
/// * `tuple_desc` - The tuple descriptor for the relation
/// * `attnum` - The attribute number (1-based column index)
///
/// # Returns
/// * `Ok(Some(i32))` - The integer value if not null
/// * `Ok(None)` - If the value is null
/// * `Err(CaliberError)` - If extraction fails
pub unsafe fn extract_i32(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
    attnum: i16,
) -> CaliberResult<Option<i32>> {
    let (datum, is_null) = unsafe { extract_datum(tuple, tuple_desc, attnum) }?;

    if is_null {
        return Ok(None);
    }

    // i32 is passed by value in PostgreSQL, so we can directly cast
    let value: i32 = unsafe { i32::from_datum(datum, false) }
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!("Failed to convert datum to i32 at column {}", attnum),
        }))?;

    Ok(Some(value))
}

/// Extract an i64 (INT8/BIGINT) value from a heap tuple.
///
/// # Arguments
/// * `tuple` - The heap tuple to extract from
/// * `tuple_desc` - The tuple descriptor for the relation
/// * `attnum` - The attribute number (1-based column index)
///
/// # Returns
/// * `Ok(Some(i64))` - The integer value if not null
/// * `Ok(None)` - If the value is null
/// * `Err(CaliberError)` - If extraction fails
pub unsafe fn extract_i64(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
    attnum: i16,
) -> CaliberResult<Option<i64>> {
    let (datum, is_null) = unsafe { extract_datum(tuple, tuple_desc, attnum) }?;

    if is_null {
        return Ok(None);
    }

    let value: i64 = unsafe { i64::from_datum(datum, false) }
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!("Failed to convert datum to i64 at column {}", attnum),
        }))?;

    Ok(Some(value))
}

/// Extract a boolean value from a heap tuple.
///
/// # Arguments
/// * `tuple` - The heap tuple to extract from
/// * `tuple_desc` - The tuple descriptor for the relation
/// * `attnum` - The attribute number (1-based column index)
///
/// # Returns
/// * `Ok(Some(bool))` - The boolean value if not null
/// * `Ok(None)` - If the value is null
/// * `Err(CaliberError)` - If extraction fails
pub unsafe fn extract_bool(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
    attnum: i16,
) -> CaliberResult<Option<bool>> {
    let (datum, is_null) = unsafe { extract_datum(tuple, tuple_desc, attnum) }?;

    if is_null {
        return Ok(None);
    }

    let value: bool = unsafe { bool::from_datum(datum, false) }
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!("Failed to convert datum to bool at column {}", attnum),
        }))?;

    Ok(Some(value))
}

/// Extract a float4 (f32) value from a heap tuple (Battle Intel Feature 1).
///
/// # Arguments
/// * `tuple` - The heap tuple to extract from
/// * `tuple_desc` - The tuple descriptor for the relation
/// * `attnum` - The attribute number (1-based column index)
///
/// # Returns
/// * `Ok(Some(f32))` - The float value if not null
/// * `Ok(None)` - If the value is null
/// * `Err(CaliberError)` - If extraction fails
pub unsafe fn extract_float4(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
    attnum: i16,
) -> CaliberResult<Option<f32>> {
    let (datum, is_null) = unsafe { extract_datum(tuple, tuple_desc, attnum) }?;

    if is_null {
        return Ok(None);
    }

    let value: f32 = unsafe { f32::from_datum(datum, false) }
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!("Failed to convert datum to float4 at column {}", attnum),
        }))?;

    Ok(Some(value))
}

/// Extract a timestamp with timezone value from a heap tuple.
///
/// # Arguments
/// * `tuple` - The heap tuple to extract from
/// * `tuple_desc` - The tuple descriptor for the relation
/// * `attnum` - The attribute number (1-based column index)
///
/// # Returns
/// * `Ok(Some(TimestampWithTimeZone))` - The timestamp value if not null
/// * `Ok(None)` - If the value is null
/// * `Err(CaliberError)` - If extraction fails
pub unsafe fn extract_timestamp(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
    attnum: i16,
) -> CaliberResult<Option<TimestampWithTimeZone>> {
    let (datum, is_null) = unsafe { extract_datum(tuple, tuple_desc, attnum) }?;

    if is_null {
        return Ok(None);
    }

    let value: TimestampWithTimeZone = unsafe { 
        TimestampWithTimeZone::from_datum(datum, false) 
    }.ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
        reason: format!("Failed to convert datum to timestamp at column {}", attnum),
    }))?;

    Ok(Some(value))
}


/// Extract a JSONB value from a heap tuple.
///
/// # Arguments
/// * `tuple` - The heap tuple to extract from
/// * `tuple_desc` - The tuple descriptor for the relation
/// * `attnum` - The attribute number (1-based column index)
///
/// # Returns
/// * `Ok(Some(serde_json::Value))` - The JSON value if not null
/// * `Ok(None)` - If the value is null
/// * `Err(CaliberError)` - If extraction fails
pub unsafe fn extract_jsonb(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
    attnum: i16,
) -> CaliberResult<Option<serde_json::Value>> {
    let (datum, is_null) = unsafe { extract_datum(tuple, tuple_desc, attnum) }?;

    if is_null {
        return Ok(None);
    }

    let jsonb: pgrx::JsonB = unsafe { pgrx::JsonB::from_datum(datum, false) }
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!("Failed to convert datum to JSONB at column {}", attnum),
        }))?;

    Ok(Some(jsonb.0))
}

/// Extract a BYTEA value from a heap tuple.
///
/// # Arguments
/// * `tuple` - The heap tuple to extract from
/// * `tuple_desc` - The tuple descriptor for the relation
/// * `attnum` - The attribute number (1-based column index)
///
/// # Returns
/// * `Ok(Some(Vec<u8>))` - The binary data if not null
/// * `Ok(None)` - If the value is null
/// * `Err(CaliberError)` - If extraction fails
pub unsafe fn extract_bytea(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
    attnum: i16,
) -> CaliberResult<Option<Vec<u8>>> {
    let (datum, is_null) = unsafe { extract_datum(tuple, tuple_desc, attnum) }?;

    if is_null {
        return Ok(None);
    }

    let bytes: Vec<u8> = unsafe { Vec::<u8>::from_datum(datum, false) }
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!("Failed to convert datum to BYTEA at column {}", attnum),
        }))?;

    Ok(Some(bytes))
}

/// Extract a FLOAT4 ARRAY (real[]) value from a heap tuple.
/// Used for embedding vectors.
///
/// # Arguments
/// * `tuple` - The heap tuple to extract from
/// * `tuple_desc` - The tuple descriptor for the relation
/// * `attnum` - The attribute number (1-based column index)
///
/// # Returns
/// * `Ok(Some(Vec<f32>))` - The float array if not null
/// * `Ok(None)` - If the value is null
/// * `Err(CaliberError)` - If extraction fails
pub unsafe fn extract_float4_array(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
    attnum: i16,
) -> CaliberResult<Option<Vec<f32>>> {
    let (datum, is_null) = unsafe { extract_datum(tuple, tuple_desc, attnum) }?;

    if is_null {
        return Ok(None);
    }

    let array: Vec<f32> = unsafe { Vec::<f32>::from_datum(datum, false) }
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!("Failed to convert datum to float4[] at column {}", attnum),
        }))?;

    Ok(Some(array))
}

/// Extract a UUID ARRAY value from a heap tuple.
/// Used for source_trajectory_ids and similar array fields.
///
/// # Arguments
/// * `tuple` - The heap tuple to extract from
/// * `tuple_desc` - The tuple descriptor for the relation
/// * `attnum` - The attribute number (1-based column index)
///
/// # Returns
/// * `Ok(Some(Vec<uuid::Uuid>))` - The UUID array if not null
/// * `Ok(None)` - If the value is null
/// * `Err(CaliberError)` - If extraction fails
pub unsafe fn extract_uuid_array(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
    attnum: i16,
) -> CaliberResult<Option<Vec<uuid::Uuid>>> {
    let (datum, is_null) = unsafe { extract_datum(tuple, tuple_desc, attnum) }?;

    if is_null {
        return Ok(None);
    }

    let pg_uuids: Vec<pgrx::Uuid> = unsafe { Vec::<pgrx::Uuid>::from_datum(datum, false) }
        .ok_or_else(|| CaliberError::Storage(StorageError::TransactionFailed {
            reason: format!("Failed to convert datum to uuid[] at column {}", attnum),
        }))?;

    // Convert pgrx::Uuid to uuid::Uuid
    let uuids: Vec<uuid::Uuid> = pg_uuids
        .into_iter()
        .map(|pg_uuid| uuid::Uuid::from_bytes(*pg_uuid.as_bytes()))
        .collect();

    Ok(Some(uuids))
}


// ============================================================================
// CONTENT HASH EXTRACTION
// ============================================================================

/// Extract a content hash (BYTEA stored as 32 bytes) from a heap tuple.
///
/// # Arguments
/// * `tuple` - The heap tuple to extract from
/// * `tuple_desc` - The tuple descriptor for the relation
/// * `attnum` - The attribute number (1-based column index)
///
/// # Returns
/// * `Ok(Some([u8; 32]))` - The content hash if not null
/// * `Ok(None)` - If the value is null
/// * `Err(CaliberError)` - If extraction fails or hash is wrong size
pub unsafe fn extract_content_hash(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
    attnum: i16,
) -> CaliberResult<Option<caliber_core::ContentHash>> {
    let bytes = extract_bytea(tuple, tuple_desc, attnum)?;

    match bytes {
        None => Ok(None),
        Some(data) => {
            if data.len() != 32 {
                return Err(CaliberError::Storage(StorageError::TransactionFailed {
                    reason: format!(
                        "Content hash at column {} has wrong size: expected 32, got {}",
                        attnum,
                        data.len()
                    ),
                }));
            }
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&data);
            Ok(Some(hash))
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS FOR ENTITY CONVERSION
// ============================================================================

/// Convert a pgrx TimestampWithTimeZone to a chrono DateTime<Utc>.
///
/// # Arguments
/// * `ts` - The pgrx timestamp
///
/// # Returns
/// A chrono DateTime<Utc>
pub fn timestamp_to_chrono(ts: TimestampWithTimeZone) -> chrono::DateTime<chrono::Utc> {
    // TimestampWithTimeZone stores microseconds since 2000-01-01 00:00:00 UTC
    // We need to convert to chrono's epoch (1970-01-01)
    
    // PostgreSQL epoch is 2000-01-01 00:00:00 UTC
    // Unix epoch is 1970-01-01 00:00:00 UTC
    // Difference is 30 years = 946684800 seconds
    const PG_EPOCH_OFFSET_SECS: i64 = 946_684_800;
    
    // Get the raw microseconds value from pgrx timestamp
    let pg_micros = ts.into_inner();
    
    // Convert to Unix timestamp (seconds + nanoseconds)
    let unix_secs = (pg_micros / 1_000_000) + PG_EPOCH_OFFSET_SECS;
    let nanos = ((pg_micros % 1_000_000) * 1000) as u32;
    
    chrono::DateTime::from_timestamp(unix_secs, nanos)
        .unwrap_or_else(chrono::Utc::now)
}

/// Convert a chrono DateTime<Utc> to a pgrx TimestampWithTimeZone.
///
/// # Arguments
/// * `dt` - The chrono datetime
///
/// # Returns
/// A pgrx TimestampWithTimeZone
pub fn chrono_to_timestamp(dt: chrono::DateTime<chrono::Utc>) -> TimestampWithTimeZone {
    // Convert chrono timestamp to PostgreSQL timestamp
    const PG_EPOCH_OFFSET_SECS: i64 = 946_684_800;

    let unix_secs = dt.timestamp();
    let nanos = dt.timestamp_subsec_nanos();

    // Convert to PostgreSQL microseconds since 2000-01-01
    let pg_micros = (unix_secs - PG_EPOCH_OFFSET_SECS) * 1_000_000 + (nanos / 1000) as i64;

    // In pgrx 0.16+, use TryFrom to create from raw TimestampTz
    TimestampWithTimeZone::try_from(pg_micros).unwrap_or_else(|_| {
        // Fallback: create from components if raw conversion fails
        TimestampWithTimeZone::new(
            dt.year(),
            dt.month() as u8,
            dt.day() as u8,
            dt.hour() as u8,
            dt.minute() as u8,
            dt.second() as f64 + dt.nanosecond() as f64 / 1_000_000_000.0,
        ).expect("chrono DateTime should produce valid timestamp")
    })
}

// ============================================================================
// DATUM CONVERSION HELPERS
// ============================================================================

/// Convert a uuid::Uuid to a pgrx Datum.
///
/// # Arguments
/// * `id` - The UUID to convert
///
/// # Returns
/// The datum representation
pub fn uuid_to_datum(id: uuid::Uuid) -> pg_sys::Datum {
    let pg_uuid = pgrx::Uuid::from_bytes(*id.as_bytes());
    pg_uuid.into_datum().unwrap_or(pg_sys::Datum::from(0))
}

/// Convert an optional uuid::Uuid to a pgrx Datum.
///
/// # Arguments
/// * `id` - The optional UUID to convert
///
/// # Returns
/// The datum representation (null datum if None)
pub fn option_uuid_to_datum(id: Option<uuid::Uuid>) -> pg_sys::Datum {
    match id {
        Some(uuid) => uuid_to_datum(uuid),
        None => pg_sys::Datum::from(0),
    }
}

/// Convert a String to a pgrx Datum.
///
/// # Arguments
/// * `s` - The string to convert
///
/// # Returns
/// The datum representation
pub fn string_to_datum(s: &str) -> pg_sys::Datum {
    s.into_datum().unwrap_or(pg_sys::Datum::from(0))
}

/// Convert an optional String to a pgrx Datum.
///
/// # Arguments
/// * `s` - The optional string to convert
///
/// # Returns
/// The datum representation (null datum if None)
pub fn option_string_to_datum(s: Option<&str>) -> pg_sys::Datum {
    match s {
        Some(str) => string_to_datum(str),
        None => pg_sys::Datum::from(0),
    }
}


/// Convert an i32 to a pgrx Datum.
///
/// # Arguments
/// * `n` - The integer to convert
///
/// # Returns
/// The datum representation
pub fn i32_to_datum(n: i32) -> pg_sys::Datum {
    n.into_datum().unwrap_or(pg_sys::Datum::from(0))
}

/// Convert an i64 to a pgrx Datum.
///
/// # Arguments
/// * `n` - The integer to convert
///
/// # Returns
/// The datum representation
pub fn i64_to_datum(n: i64) -> pg_sys::Datum {
    n.into_datum().unwrap_or(pg_sys::Datum::from(0))
}

/// Convert a bool to a pgrx Datum.
///
/// # Arguments
/// * `b` - The boolean to convert
///
/// # Returns
/// The datum representation
pub fn bool_to_datum(b: bool) -> pg_sys::Datum {
    b.into_datum().unwrap_or(pg_sys::Datum::from(0))
}

/// Convert a float4 (f32) to a pgrx Datum (Battle Intel Feature 1).
///
/// # Arguments
/// * `f` - The float to convert
///
/// # Returns
/// The datum representation
pub fn float4_to_datum(f: f32) -> pg_sys::Datum {
    f.into_datum().unwrap_or(pg_sys::Datum::from(0))
}

/// Convert a serde_json::Value to a pgrx JSONB Datum.
///
/// # Arguments
/// * `json` - The JSON value to convert
///
/// # Returns
/// The datum representation
pub fn json_to_datum(json: &serde_json::Value) -> pg_sys::Datum {
    pgrx::JsonB(json.clone()).into_datum().unwrap_or(pg_sys::Datum::from(0))
}

/// Convert an optional serde_json::Value to a pgrx JSONB Datum.
///
/// # Arguments
/// * `json` - The optional JSON value to convert
///
/// # Returns
/// The datum representation (null datum if None)
pub fn option_json_to_datum(json: Option<&serde_json::Value>) -> pg_sys::Datum {
    match json {
        Some(j) => json_to_datum(j),
        None => pg_sys::Datum::from(0),
    }
}

/// Convert a Vec<u8> to a pgrx BYTEA Datum.
///
/// # Arguments
/// * `bytes` - The bytes to convert
///
/// # Returns
/// The datum representation
pub fn bytea_to_datum(bytes: &[u8]) -> pg_sys::Datum {
    bytes.to_vec().into_datum().unwrap_or(pg_sys::Datum::from(0))
}

/// Convert a content hash ([u8; 32]) to a pgrx BYTEA Datum.
///
/// # Arguments
/// * `hash` - The content hash to convert
///
/// # Returns
/// The datum representation
pub fn content_hash_to_datum(hash: &caliber_core::ContentHash) -> pg_sys::Datum {
    bytea_to_datum(hash)
}

/// Convert a Vec<f32> to a pgrx FLOAT4 ARRAY Datum.
///
/// # Arguments
/// * `floats` - The float array to convert
///
/// # Returns
/// The datum representation
pub fn float4_array_to_datum(floats: &[f32]) -> pg_sys::Datum {
    floats.to_vec().into_datum().unwrap_or(pg_sys::Datum::from(0))
}

/// Convert a Vec<uuid::Uuid> to a pgrx UUID ARRAY Datum.
///
/// # Arguments
/// * `uuids` - The UUID array to convert
///
/// # Returns
/// The datum representation
pub fn uuid_array_to_datum(uuids: &[uuid::Uuid]) -> pg_sys::Datum {
    let pg_uuids: Vec<pgrx::Uuid> = uuids
        .iter()
        .map(|u| pgrx::Uuid::from_bytes(*u.as_bytes()))
        .collect();
    pg_uuids.into_datum().unwrap_or(pg_sys::Datum::from(0))
}

/// Convert a chrono DateTime<Utc> to a pgrx Datum.
///
/// # Arguments
/// * `dt` - The datetime to convert
///
/// # Returns
/// The datum representation
pub fn datetime_to_datum(dt: chrono::DateTime<chrono::Utc>) -> pg_sys::Datum {
    chrono_to_timestamp(dt).into_datum().unwrap_or(pg_sys::Datum::from(0))
}

/// Convert an optional chrono DateTime<Utc> to a pgrx Datum.
///
/// # Arguments
/// * `dt` - The optional datetime to convert
///
/// # Returns
/// The datum representation (null datum if None)
pub fn option_datetime_to_datum(dt: Option<chrono::DateTime<chrono::Utc>>) -> pg_sys::Datum {
    match dt {
        Some(d) => datetime_to_datum(d),
        None => pg_sys::Datum::from(0),
    }
}

/// Convert a slice of Strings to a PostgreSQL TEXT[] datum.
///
/// # Arguments
/// * `texts` - The string slice to convert
///
/// # Returns
/// The datum representation
pub fn text_array_to_datum(texts: &[String]) -> pg_sys::Datum {
    let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
    text_refs.into_datum().unwrap_or(pg_sys::Datum::from(0))
}

/// Extract a TEXT[] (string array) from a heap tuple.
///
/// # Arguments
/// * `tuple` - The heap tuple to extract from
/// * `tuple_desc` - The tuple descriptor
/// * `attnum` - The attribute number (1-based column index)
///
/// # Returns
/// * `Ok(Some(Vec<String>))` - The string array if not null
/// * `Ok(None)` - If the value is NULL
/// * `Err(CaliberError)` - On extraction failure
pub unsafe fn extract_text_array(
    tuple: *mut pg_sys::HeapTupleData,
    tuple_desc: pg_sys::TupleDesc,
    attnum: i16,
) -> CaliberResult<Option<Vec<String>>> {
    let (datum, is_null) = unsafe { extract_datum(tuple, tuple_desc, attnum) }?;
    
    if is_null {
        return Ok(None);
    }
    
    // Convert datum to Vec<String>
    let array: Option<Vec<String>> = unsafe {
        FromDatum::from_datum(datum, false)
    };
    
    Ok(array)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_conversion_roundtrip() {
        let now = chrono::Utc::now();
        let pg_ts = chrono_to_timestamp(now);
        let back = timestamp_to_chrono(pg_ts);
        
        // Allow for microsecond precision loss
        let diff = (now.timestamp_micros() - back.timestamp_micros()).abs();
        assert!(diff < 2, "Timestamp roundtrip should preserve microsecond precision");
    }

    #[test]
    fn test_uuid_to_datum_roundtrip() {
        let original = uuid::Uuid::new_v4();
        let datum = uuid_to_datum(original);
        
        // Convert back
        let pg_uuid: pgrx::Uuid = unsafe { pgrx::Uuid::from_datum(datum, false) }.unwrap();
        let back = uuid::Uuid::from_bytes(*pg_uuid.as_bytes());
        
        assert_eq!(original, back);
    }
}
