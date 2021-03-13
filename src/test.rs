use std::sync::Once;

#[allow(dead_code)]
static INIT: Once = Once::new();

#[cfg(test)]
#[ctor::ctor]
fn initialize_test() {
    INIT.call_once(|| {
        //let mut container = Container::<profiles::Default>::new();
    });
}