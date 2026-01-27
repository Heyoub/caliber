//! Validation Traits
//!
//! Common validation patterns extracted from route handlers.
//! These traits reduce boilerplate and improve consistency.

use crate::error::{ApiError, ApiResult};

/// Trait for validating non-empty strings.
///
/// # Example
/// ```ignore
/// use caliber_api::validation::ValidateNonEmpty;
///
/// fn create_user(name: &str) -> ApiResult<()> {
///     name.validate_non_empty("name")?;
///     // ... rest of logic
/// }
/// ```
pub trait ValidateNonEmpty {
    /// Validate that the value is non-empty.
    ///
    /// # Arguments
    /// - `field_name`: Name of the field for error messages
    ///
    /// # Errors
    /// Returns `ApiError::missing_field` if the value is empty or whitespace-only.
    fn validate_non_empty(&self, field_name: &str) -> ApiResult<()>;
}

impl ValidateNonEmpty for str {
    fn validate_non_empty(&self, field_name: &str) -> ApiResult<()> {
        if self.trim().is_empty() {
            return Err(ApiError::missing_field(field_name));
        }
        Ok(())
    }
}

impl ValidateNonEmpty for &str {
    fn validate_non_empty(&self, field_name: &str) -> ApiResult<()> {
        (*self).validate_non_empty(field_name)
    }
}

impl ValidateNonEmpty for String {
    fn validate_non_empty(&self, field_name: &str) -> ApiResult<()> {
        self.as_str().validate_non_empty(field_name)
    }
}

impl<T: ValidateNonEmpty> ValidateNonEmpty for Option<T> {
    fn validate_non_empty(&self, field_name: &str) -> ApiResult<()> {
        match self {
            Some(value) => value.validate_non_empty(field_name),
            None => Err(ApiError::missing_field(field_name)),
        }
    }
}

/// Trait for validating numeric ranges.
///
/// # Example
/// ```ignore
/// use caliber_api::validation::ValidateRange;
///
/// fn set_timeout(timeout_ms: i64) -> ApiResult<()> {
///     timeout_ms.validate_positive("timeout_ms")?;
///     timeout_ms.validate_range("timeout_ms", 1, 300_000)?;
///     // ... rest of logic
/// }
/// ```
pub trait ValidateRange {
    /// Validate that the value is positive (> 0).
    fn validate_positive(&self, field_name: &str) -> ApiResult<()>;

    /// Validate that the value is within an inclusive range.
    fn validate_range(&self, field_name: &str, min: Self, max: Self) -> ApiResult<()>
    where
        Self: Sized;
}

macro_rules! impl_validate_range {
    ($($t:ty),*) => {
        $(
            impl ValidateRange for $t {
                fn validate_positive(&self, field_name: &str) -> ApiResult<()> {
                    if *self <= 0 as $t {
                        return Err(ApiError::invalid_range(field_name, 1, <$t>::MAX as i64));
                    }
                    Ok(())
                }

                fn validate_range(&self, field_name: &str, min: Self, max: Self) -> ApiResult<()> {
                    if *self < min || *self > max {
                        return Err(ApiError::invalid_range(field_name, min as i64, max as i64));
                    }
                    Ok(())
                }
            }
        )*
    };
}

impl_validate_range!(i8, i16, i32, i64, isize);
impl_validate_range!(u8, u16, u32, u64, usize);

/// Trait for checking if an update request has any fields set.
///
/// Implement this on update request types to provide a unified
/// "has any updates" check.
///
/// # Example
/// ```ignore
/// use caliber_api::validation::HasUpdates;
///
/// impl HasUpdates for UpdateNoteRequest {
///     fn has_any_updates(&self) -> bool {
///         self.title.is_some()
///             || self.content.is_some()
///             || self.metadata.is_some()
///     }
/// }
///
/// fn update_note(req: UpdateNoteRequest) -> ApiResult<()> {
///     req.validate_has_updates()?;
///     // ... rest of logic
/// }
/// ```
pub trait HasUpdates {
    /// Check if any update fields are set.
    fn has_any_updates(&self) -> bool;

    /// Validate that at least one update field is set.
    fn validate_has_updates(&self) -> ApiResult<()> {
        if !self.has_any_updates() {
            return Err(ApiError::invalid_input(
                "At least one field must be provided for update",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_non_empty_str() {
        assert!("hello".validate_non_empty("test").is_ok());
        assert!("".validate_non_empty("test").is_err());
        assert!("   ".validate_non_empty("test").is_err());
        assert!("  hi  ".validate_non_empty("test").is_ok());
    }

    #[test]
    fn test_validate_non_empty_string() {
        assert!(String::from("hello").validate_non_empty("test").is_ok());
        assert!(String::from("").validate_non_empty("test").is_err());
    }

    #[test]
    fn test_validate_non_empty_option() {
        let some_str: Option<&str> = Some("hello");
        let some_empty: Option<&str> = Some("");
        let none_str: Option<&str> = None;

        assert!(some_str.validate_non_empty("test").is_ok());
        assert!(some_empty.validate_non_empty("test").is_err());
        assert!(none_str.validate_non_empty("test").is_err());
    }

    #[test]
    fn test_validate_positive() {
        assert!(5i32.validate_positive("test").is_ok());
        assert!(1i32.validate_positive("test").is_ok());
        assert!(0i32.validate_positive("test").is_err());
        assert!((-1i32).validate_positive("test").is_err());
    }

    #[test]
    fn test_validate_range() {
        assert!(5i32.validate_range("test", 1, 10).is_ok());
        assert!(1i32.validate_range("test", 1, 10).is_ok());
        assert!(10i32.validate_range("test", 1, 10).is_ok());
        assert!(0i32.validate_range("test", 1, 10).is_err());
        assert!(11i32.validate_range("test", 1, 10).is_err());
    }
}
