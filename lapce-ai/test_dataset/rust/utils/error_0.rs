use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError_0 {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Not found")]
    NotFound,
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

pub type Result<T> = std::result::Result<T, AppError_0>;

pub fn handle_error_0_gracefully(e: AppError_0) {
    match e {
        AppError_0::Database(db_err) => {
            log::error!("Database error: {:?}", db_err);
        }
        AppError_0::NotFound => {
            log::warn!("Resource not found");
        }
        _ => {}
    }
}