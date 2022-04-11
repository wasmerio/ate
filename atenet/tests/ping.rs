mod common;

#[test]
fn ping() {
    common::run(async move {
        let _servers = common::setup().await;
    })
}