// This is a stub library file for the wasmlanche-examples crate
// The actual example code is in the bin targets

// Re-export the necessary items from wasmlanche for the examples
pub use wasmlanche::{
    borsh,
    context,
    error,
    events,
    future,
    // public module doesn't exist, removing it
    state,
    types,
};

// Adding a main function to make it compile
fn main() {
    println!("This is a library example. Run individual examples instead.");
}
