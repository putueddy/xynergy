pub mod allocation;
pub mod audit_log;
pub mod auth;
pub mod department;
pub mod holiday;
pub mod project;
pub mod resource;
pub mod user;

pub use allocation::allocation_routes;
pub use audit_log::audit_log_routes;
pub use auth::{auth_routes, Claims};
pub use department::department_routes;
pub use holiday::holiday_routes;
pub use project::project_routes;
pub use resource::resource_routes;
pub use user::user_routes;
