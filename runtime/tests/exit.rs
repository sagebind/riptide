#[tokio::test]
async fn exit_throws_unrecoverable_exception() {
    match riptide_runtime::eval(r#"
        import 'builtins' for *

        try {
            exit 1
        } {
            # Attempt to recover
            println "recovered!"
        }
    "#).await {
        Ok(value) => panic!("did not expect {:?}", value),
        Err(e) => assert_eq!(e.message(), 1f64),
    }
}
