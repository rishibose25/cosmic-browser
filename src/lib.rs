// src/lib.rs

// Main modules
mod module1;
mod module2;

// Error types
#[derive(Debug)]
pub enum CosmicError {
    NotFound,
    BadRequest,
    Unauthorized,
    InternalServerError,
}

impl std::fmt::Display for CosmicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for CosmicError {}