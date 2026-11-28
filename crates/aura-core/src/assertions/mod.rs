//! Declarative / No-Code Assertions (Bruno-style `assert` rules) and performance bounds.

pub mod evaluator;
pub mod parser;
pub mod performance;
pub mod types;

pub use evaluator::{evaluate_assertions, evaluate_flow_condition};
pub(crate) use evaluator::traverse_json_path;
pub use parser::{
    parse_assert_block, parse_assertion_kv, parse_assertion_line, serialize_assertion_kv,
};
pub use performance::{PerformanceAssertionConfig, evaluate_performance_assertions};
pub use types::{AssertOperator, DeclarativeAssertion};
