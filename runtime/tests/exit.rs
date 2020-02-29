#[tokio::test]
async fn exit_throws_unrecoverable_exception() {
    let result = riptide_runtime::eval(r#"
        try {
            exit 1
        } {
            # Attempt to recover
            println "recovered!"
        }
    "#).await;

    match result {
        Ok(value) => panic!("did not expect {:?}", value),
        Err(e) => assert_eq!(e.message(), 1f64),
    }
}
