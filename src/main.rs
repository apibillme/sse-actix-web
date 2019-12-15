use sse_actix_web::{ broadcast, Broadcaster, new_client };
use actix_web::{ web, HttpServer, HttpResponse, App, Responder };

async fn index() -> impl Responder {
    let content = include_str!("index.html");

    HttpResponse::Ok()
        .header("content-type", "text/html")
        .body(content)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let data = Broadcaster::create();

    HttpServer::new(move || {
        App::new()
            .register_data(data.clone())
            .route("/", web::get().to(index))
            .route("/events", web::get().to(new_client))
            .route("/broadcast/{msg}", web::get().to(broadcast))
    })
    .bind("0.0.0.0:3000")?
    .start()
    .await
}
