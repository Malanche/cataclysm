use criterion::*;
use cataclysm::{Server, Branch, http::{Response, Method}};

async fn load() {
    let _r = reqwest::get("http://127.0.0.1:8000/").await.unwrap().text().await.unwrap();
}

async fn index() -> Response {
    Response::ok().body("hello")
}

fn bench(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let handle = rt.handle();
    handle.spawn(async {
        let path = Branch::new("/").with(Method::Get.to(index));
        
        let server = Server::builder(
            path
        ).build();
        
        server.run("127.0.0.1:8000").await.unwrap();
    });

    for number in [1, 2, 4] {
        c.bench_function(&format!("{} Actor(s), Empty Ping Pong", number), |b| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            b.to_async(rt).iter(|| async {
                let futs = (0..number).map(|_| load()).collect::<Vec<_>>();
    
                let _res = futures::future::join_all(futs).await;
            });

        });
    }
    rt.shutdown_background();
}

criterion_group!(benches, bench);
criterion_main!(benches);