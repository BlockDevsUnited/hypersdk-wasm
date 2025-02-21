use wasmlanche::{
    Context,
    host::{Host, HostState},
    types::WasmlAddress,
};
use std::sync::Arc;
use tokio::sync::RwLock;

pub fn create_test_context() -> Context {
    let state = Arc::new(RwLock::new(HostState::default()));
    let host = Host::new(state);
    let actor = WasmlAddress::try_from(&[0u8; 33][..]).unwrap();
    Context::new(actor, 0, 0, Arc::new(RwLock::new(host)), None)
}
