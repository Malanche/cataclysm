use futures::future::FutureExt;
use cataclysm::{Server, Path, Pipeline, http::{Response, Request, Method}, SimpleLogger};

async fn hello() -> Response {
    log::info!("hello callback called!");
    Response::ok().body("hello")
}

async fn world() -> Response {
    log::info!("world callback called!");
    Response::ok().body("world!")
}

// #[tokio::main(flavor = "multi_thread", worker_threads = 10)]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    SimpleLogger::new().with_level(log::LevelFilter::Info).init().unwrap();
    
    let path = Path::new("/")
        .with(Method::Get.to(hello))
        .with(Method::Post.to(world))
        .layer(|req: Request, pipeline: Box<Pipeline>| async {
            // Example of timing middleware
            log::info!("Time measuring begins");
            let now = std::time::Instant::now();
            let request = pipeline.execute(req).await;
            let elapsed = now.elapsed().as_nanos();
            log::info!("Process time: {} ns", elapsed);
            request
        }.boxed());

    let concr = format!("{}", path);
    for line in concr.split("\n") {
        log::info!("{}", line);
    }

    let server = Server::builder(
        path
    ).build();

    /*
    let server = Server::builder(
        Path::new("/").with(Method::Get.to(hello)).with(Method::Post.to(world)).defaults_to(|| async {
            Response::ok().body("Perdido?")
        })
    ).build();


    let server = Server::builder(
        Path::new("/").with(Method::Get.to(hello))
            .nested(Path::new("/world")
                .with(Method::Get.to(world)))
            .nested(Path::new("/data")
                .with(Method::Post.to(|data: Vec<u8>| async {
                    log::info!("{}", String::from_utf8(data).unwrap());
                    Response::ok().body("haha")
                }))
        )
    ).build();
    */

    server.run("localhost:8000").await.unwrap();
}