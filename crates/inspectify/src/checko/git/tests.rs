#[tokio::test]
async fn latest_commit_before_without_ignore() {
    let res = super::latest_commit_before(
        dunce::canonicalize(".").unwrap(),
        chrono::Utc::now().fixed_offset(),
        &[],
    )
    .await
    .unwrap();
    assert!(res.is_some_and(|s| !s.is_empty()));
}

#[tokio::test]
async fn latest_commit_before_with_ignore() {
    let res = super::latest_commit_before(
        dunce::canonicalize(".").unwrap(),
        chrono::Utc::now().fixed_offset(),
        &["some username that is not part of the git history".to_string()],
    )
    .await
    .unwrap();
    assert!(res.is_some_and(|s| !s.is_empty()));
}
