pub mod api;
pub mod builtins;
pub mod bus;
pub mod eval;
pub mod fs;

pub mod bin_factory;
pub mod cconst;
pub mod common;
pub mod console;
pub mod environment;
pub mod err;
pub mod fd;
pub mod job;
pub mod pipe;
pub mod poll;
pub mod reactor;
pub mod state;
pub mod stdio;
pub mod stdout;
pub mod tty;
pub mod wasi;
pub mod wizard_executor;

// Re-exports
pub use wasmer_os_grammar as grammar;
pub use grammar::ast;
pub use wasmer;
#[cfg(feature = "wasmer-compiler")]
pub use wasmer_compiler;
#[cfg(feature = "wasmer-compiler-cranelift")]
pub use wasmer_compiler_cranelift;
#[cfg(feature = "wasmer-compiler-llvm")]
pub use wasmer_compiler_llvm;
#[cfg(feature = "wasmer-compiler-singlepass")]
pub use wasmer_compiler_singlepass;
pub use wasmer_vfs;
pub use wasmer_wasi;

#[cfg(all(not(feature = "sys"), not(feature = "js")))]
compile_error!("At least the `sys` or the `js` feature must be enabled. Please, pick one.");
