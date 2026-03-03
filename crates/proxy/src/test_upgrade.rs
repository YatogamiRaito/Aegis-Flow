use hyper::{Request, Response};
use hyper::upgrade::Upgraded;

pub async fn extract<B>(req: Request<B>) {
    let upgraded: Upgraded = hyper::upgrade::on(req).await.unwrap();
}
