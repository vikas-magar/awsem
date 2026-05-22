#[macro_export]
macro_rules! lock_db {
    ($state:expr) => {
        match $state.db.lock() {
            Ok(c) => c,
            Err(e) => return $crate::error::AwsemError::Internal(e.to_string()).to_response(),
        }
    };
}
