pub async fn test_reqwest_upgrade(res: reqwest::Response) {
    let upgraded = res.upgrade().await.unwrap();
}
