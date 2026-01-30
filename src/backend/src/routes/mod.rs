pub mod auth;
pub mod department;
pub mod user;
pub mod resource;
pub mod project;
pub mod allocation;

pub use auth::{auth_routes, Claims};
pub use department::department_routes;
pub use user::user_routes;
pub use resource::resource_routes;
pub use project::project_routes;
pub use allocation::allocation_routes;
