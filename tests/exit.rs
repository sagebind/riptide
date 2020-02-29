use riptide::runtime;

#[tokio::test]
async fn exit_throws_unrecoverable_exception() {
    let mut fiber = runtime::init().await.unwrap();

    let result = fiber.execute(None, r#"
        try {
            exit 1
        } {
            println "recovered!"
        }
    "#).await;

    match result {
        Ok(value) => panic!("did not expect {:?}", value),
        Err(e) => assert_eq!(e.message(), 1f64),
    }
}
