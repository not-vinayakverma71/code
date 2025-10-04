use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError_10 {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Not found")]
    NotFound,
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

pub type Result<T> = std::result::Result<T, AppError_10>;

pub fn handle_error_10_gracefully(e: AppError_10) {
    match e {
        AppError_10::Database(db_err) => {
            log::error!("Database error: {:?}", db_err);
        }
        AppError_10::NotFound => {
            log::warn!("Resource not found");
        }
        _ => {}
    }
}