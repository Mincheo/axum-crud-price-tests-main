use std::{
    collections::HashMap,
    sync::Arc,
};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
    response::IntoResponse,
    Router,
    routing::*,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

type GlobalPrice = u64;
type GlobalPriceMap = Arc<RwLock<HashMap<Uuid, GlobalPrice>>>;

#[tokio::main]
async fn main() {
let global_price = Arc::new(RwLock::new(HashMap::default()));
let app = app(global_price);

let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
println!("Listening on 127.0.0.1:3000");
axum::serve(listener, app).await.unwrap();

}

fn app(state: GlobalPriceMap) -> Router {
    Router::new()
        .route("/price", get(get_price_all).post(create_price))
        .route("/price/:id", get(get_price_by_id).patch(upd_price).delete(del_price))
        .with_state(state)
}

#[derive(Debug, Serialize, Deserialize)]
struct PriceStruct {
    price: GlobalPrice,
}

async fn create_price(
    State(prices): State<GlobalPriceMap>,
    Json(input): Json<PriceStruct>,
) -> Result<impl IntoResponse, StatusCode> {
    let uuid = Uuid::new_v4();
    prices.write().await.insert(uuid, input.price);

    Ok(uuid.to_string())
}

async fn get_price_all(
	State(global_price): State<GlobalPriceMap>,
) -> Result<impl IntoResponse, StatusCode> {
	let global_price = global_price.read().await;
		Ok(Json(global_price.values().cloned().collect::<Vec<GlobalPrice>>()))
}

async fn get_price_by_id(
    Path(id): Path<Uuid>,
    State(prices): State<GlobalPriceMap>,
) -> Result<impl IntoResponse, StatusCode>{
    match prices.read().await.get(&id) {
        Some(price) => Ok(price.to_string()),
        None => Err(StatusCode::NOT_FOUND)
    } 
}

async fn upd_price(
    Path(id): Path<Uuid>,
    State(prices): State<GlobalPriceMap>,
    Json(input): Json<PriceStruct>,
) -> Result<impl IntoResponse, StatusCode> {
    match prices.write().await.get_mut(&id) {
        Some(price_old) => {
            *price_old = input.price;
            Ok(StatusCode::OK)
        },
        None => Err(StatusCode::NOT_FOUND)
    }
}

async fn del_price(
    Path(id): Path<Uuid>,
    State(prices): State<GlobalPriceMap>,
)   -> Result<impl IntoResponse, StatusCode> {
    match prices.write().await.remove_entry(&id) {
        Some(_) => Ok(StatusCode::OK),
        None => Err(StatusCode::NOT_FOUND)
    }
}



#[cfg(test)]
mod tests{
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };

    use axum::response::Response;
    use axum::routing::RouterIntoService;
    use http_body_util::BodyExt;
    use serde_json::Value;
    use tower::{Service, ServiceExt};
    use serde_json::json;

    use super::*;


    #[tokio::test]
    async fn test_get_price_all_ok(){
        let uuid = Uuid::new_v4();
        let mut hash_price: HashMap<Uuid, GlobalPrice> = HashMap::new();
        hash_price.insert(uuid, 14);

        let state = Arc::new(RwLock::new(hash_price));
        let mut app = app(state).into_service();

        let request = build_request(
            http::Method::GET,
            "/price",
            None
        );


        let response = ServiceExt::<Request<Body>>::ready(&mut app)
        .await
        .unwrap()
        .call(request)
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.into_body().collect().await.unwrap().to_bytes(), "[14]");

    }

    #[tokio::test]
    async fn test_get_price_by_id_not_found(){
        let uuid = Uuid::new_v4();
        let mut hash_price: HashMap<Uuid, GlobalPrice> = HashMap::new();
        hash_price.insert(uuid, 14);

        let state = Arc::new(RwLock::new(hash_price));
        let mut app = app(state).into_service();
        let request = build_request(
            http::Method::GET,
            &format!("/price/{}", Uuid::new_v4()),
            None
        );
        let response = ServiceExt::<Request<Body>>::ready(&mut app)
        .await
        .unwrap()
        .call(request)
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response.into_body().collect().await.unwrap().to_bytes(), "");

    }
    #[tokio::test]
    async fn test_get_price_by_id(){

        let uuid = Uuid::new_v4();
        let mut hash_price: HashMap<Uuid, GlobalPrice> = HashMap::new();
        hash_price.insert(uuid, 14);

        let state = Arc::new(RwLock::new(hash_price));
        let mut app = app(state).into_service();
        let request = build_request(
            http::Method::GET,
            &format!("/price/{}", uuid),
            None
        );
        let response = ServiceExt::<Request<Body>>::ready(&mut app)
        .await
        .unwrap()
        .call(request)
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.into_body().collect().await.unwrap().to_bytes(), "14");
    }
    #[tokio::test]
    async fn test_upd_price(){
        let uuid = Uuid::new_v4();
        let mut hash_price: HashMap<Uuid, GlobalPrice> = HashMap::new();
        hash_price.insert(uuid, 10);

        let state = Arc::new(RwLock::new(hash_price));
        let mut app = app(state).into_service();

        let request = build_request(
            http::Method::PATCH,
            &format!("/price/{}", uuid),
            Some(&json!({"price": 66}))
        );
        let _response1 = ServiceExt::<Request<Body>>::ready(&mut app)
        .await
        .unwrap()
        .call(request)
        .await
        .unwrap();

        assert_eq!(_response1.status(),StatusCode::OK);

        let request = build_request(
            http::Method::GET,
            &format!("/price/{}", uuid),
            None
        );
        let _response2 = ServiceExt::<Request<Body>>::ready(&mut app)
        .await
        .unwrap()
        .call(request)
        .await
        .unwrap();

        assert_eq!(_response2.into_body().collect().await.unwrap().to_bytes(), "66");
        }
    
    #[tokio::test]
    async fn test_del_price(){
        let uuid = Uuid::new_v4();
        let mut hash_price: HashMap<Uuid, GlobalPrice> = HashMap::new();
        hash_price.insert(uuid, 10);

        let state = Arc::new(RwLock::new(hash_price));
        let mut app = app(state).into_service();

        let request = build_request(
            http::Method::DELETE,
            &format!("/price/{}", uuid),
            None
        );
        let resp = ServiceExt::<Request<Body>>::ready(&mut app)
        .await
        .unwrap()
        .call(request)
        .await
        .unwrap();

        assert_eq!(resp.status(),StatusCode::OK);

    }


fn build_request(method: http::Method, uri: &str, json: Option<&Value>) -> Request<Body> {
    let body = match json {
        Some(json) => Body::from(
            serde_json::to_vec(json).unwrap(),
        ),
        None => Body::empty(),
    };

    Request::builder()
        .method(method)
        .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
        .uri(uri)
        .body(body)
        .unwrap()
}

}
