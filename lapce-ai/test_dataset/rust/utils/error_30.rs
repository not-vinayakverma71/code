use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError_30 {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Not found")]
    NotFound,
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

pub type Result<T> = std::result::Result<T, AppError_30>;

pub fn handle_error_30_gracefully(e: AppError_30) {
    match e {
        AppError_30::Database(db_err) => {
            log::error!("Database error: {:?}", db_err);
        }
        AppError_30::NotFound => {
            log::warn!("Resource not found");
        }
        _ => {}
    }
}