extern crate build_deps;

fn main() {
    build_deps::rerun_if_changed_paths( "static/bin/*" ).unwrap();    
    build_deps::rerun_if_changed_paths( "static/dev/*" ).unwrap();
    build_deps::rerun_if_changed_paths( "static/etc/*" ).unwrap();
    build_deps::rerun_if_changed_paths( "static/tmp/*" ).unwrap();
    build_deps::rerun_if_changed_paths( "static/*" ).unwrap();
}