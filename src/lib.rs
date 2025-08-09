// Core domain layer
pub mod core;

// Application layer
pub mod application;

// Infrastructure layer
pub mod infrastructure;

// File transfer protocol layer
pub mod file_transfer;

// Protocol module for backward compatibility
pub mod protocol;

// Re-export specific items to avoid ambiguous glob re-exports
pub use core::domain::*;
pub use core::traits::*;
pub use application::{UseCases, ApplicationService, FileSystemService};
pub use infrastructure::{AppConfig, NetworkServiceImpl, CryptoService, UtilityService, RepositoryBuilder};
// Protocol re-exports for external users
pub use file_transfer::{FileTransferCodec, FileTransferProtocol};

// Re-export crypto module for backward compatibility with tests
pub use core::crypto; 