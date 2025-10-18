use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError_40 {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Not found")]
    NotFound,
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

pub type Result<T> = std::result::Result<T, AppError_40>;

pub fn handle_error_40_gracefully(e: AppError_40) {
    match e {
        AppError_40::Database(db_err) => {
            log::error!("Database error: {:?}", db_err);
        }
        AppError_40::NotFound => {
            log::warn!("Resource not found");
        }
        _ => {}
    }
}