use cataclysm::{Server, Branch, http::{Response, Method}};

#[tokio::test]
async fn path() {
    async fn index() -> Response {
        Response::ok().body("hello")
    }

    let _jh = tokio::spawn(async {
        let branch: Branch<()> = Branch::new("/some/long/path").with(Method::Get.to(index));
        let server = Server::builder(branch).build().unwrap();
        server.run("127.0.0.1:8000").await.unwrap();
    });

    let response = reqwest::get("http://127.0.0.1:8000/some/long/path").await.unwrap().text().await.unwrap();

    assert_eq!(response, "hello");
}

#[tokio::test]
async fn timeout() {
    async fn index() -> Response {
        tokio::time::sleep(tokio::time::Duration::from_millis(2_000)).await;
        Response::ok().body("hello")
    }

    let _jh = tokio::spawn(async {
        let branch: Branch<()> = Branch::new("/some/long/path").with(Method::Get.to(index));
        let server = Server::builder(branch).timeout(std::time::Duration::from_millis(1_000)).build().unwrap();
        server.run("127.0.0.1:8001").await.unwrap();
    });

    let response = reqwest::get("http://127.0.0.1:8001/some/long/path").await;

    assert!(response.is_err());
}


#[tokio::test]
async fn max_connections() {
    async fn index() -> Response {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        Response::ok().body("hello")
    }

    let _jh = tokio::spawn(async {
        let branch: Branch<()> = Branch::new("/").with(Method::Get.to(index));
        let server = Server::builder(branch).max_connections(10).build().unwrap();
        server.run("127.0.0.1:8002").await.unwrap();
    });

    let vals: Vec<_> = (0..30).map(|_| reqwest::get("http://127.0.0.1:8002/")).collect();
    let now = std::time::Instant::now();
    let _: Vec<_> = futures::future::join_all(vals).await.into_iter().map(|v| v.unwrap().status()).collect();
    assert!(now.elapsed().as_millis() > 1_499);
}