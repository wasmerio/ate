extern crate build_deps;

fn main() {
    #[cfg(feature = "embedded_files")]
    build_deps::rerun_if_changed_paths( "public/bin/*" ).unwrap();
    #[cfg(feature = "embedded_files")]
    build_deps::rerun_if_changed_paths( "public/*" ).unwrap();
    #[cfg(feature = "embedded_files")]
    build_deps::rerun_if_changed_paths( "public/bin" ).unwrap();
    #[cfg(feature = "embedded_files")]
    build_deps::rerun_if_changed_paths( "public" ).unwrap();
}