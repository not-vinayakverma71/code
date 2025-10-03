use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError_20 {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Not found")]
    NotFound,
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

pub type Result<T> = std::result::Result<T, AppError_20>;

pub fn handle_error_20_gracefully(e: AppError_20) {
    match e {
        AppError_20::Database(db_err) => {
            log::error!("Database error: {:?}", db_err);
        }
        AppError_20::NotFound => {
            log::warn!("Resource not found");
        }
        _ => {}
    }
}