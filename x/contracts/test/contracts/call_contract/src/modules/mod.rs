// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

pub mod simple_call;
pub mod actor_check;
pub mod call_with_param;
pub mod call_with_two_params;

// Re-export all the functions
pub use simple_call::*;
pub use actor_check::*;
pub use call_with_param::*;
pub use call_with_two_params::*;
