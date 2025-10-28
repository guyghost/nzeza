pub mod entities;
pub mod errors;
pub mod repositories;
pub mod services;
pub mod value_objects;

// TDD RED Phase: Comprehensive test specifications
#[cfg(test)]
mod errors_tests;
#[cfg(test)]
mod position_validation_tests;
#[cfg(test)]
mod order_execution_tests;
#[cfg(test)]
mod portfolio_consistency_tests;
#[cfg(test)]
mod concurrency_tests;
