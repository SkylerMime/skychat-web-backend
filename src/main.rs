use futures::StreamExt;
use mongodb::bson::{doc, DateTime};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{self, routes};
use skyserver::{get_messages_collection, ChatMessage};

use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Request, Response};

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

#[rocket::get("/")]
async fn index() -> &'static str {
    "Hello, world!"
}

#[rocket::get("/user/<name>")]
async fn user(name: &str) -> String {
    let retrieved_user = skyserver::get_user(name).await;
    format!("User's name: {}", retrieved_user.name)
}

#[rocket::post("/messages-handler", data = "<message>")]
async fn messages_poster(message: Json<ChatMessage>) -> (Status, String) {
    match skyserver::put_message(message.into_inner()).await {
        Ok(response) => (Status::Ok, format!("New message with id: {:?}", response)),
        Err(error) => (
            Status::NotFound,
            format!("Problem adding message to database: {:?}", error),
        ),
    }
}

#[rocket::get("/messages-handler")]
async fn messages_getter() -> Json<Vec<ChatMessage>> {
    Json(skyserver::get_messages().await)
}

#[rocket::get("/messages-stream")]
fn messages_stream(ws: ws::WebSocket) -> ws::Stream!['static] {
    ws::Stream! { ws =>
        let example_message: ChatMessage = ChatMessage {username: String::from("Server"),
            message: String::from("Welcome back to the chat stream!"),
            datetime: DateTime::now()};
        let message_json_string: String = serde_json::to_string(&example_message).expect("Should sucessfully serialize");
        yield message_json_string.into();

        // Main loop for tracking new messages
        let insert_events_pipeline = vec![ doc! { "$match": doc! { "operationType": "insert" } } ];
        let mut new_documents_stream = get_messages_collection().await.watch(insert_events_pipeline, None).await.expect("Collection should be watchable");

        while let Some(insert_event) = new_documents_stream.next().await.transpose().expect("Should successfuly transpose") {
            let message = insert_event.full_document;
            yield serde_json::to_string(&message).expect("ChatMessage should successfully serialize").into();
        }
    }
}

#[rocket::launch]
fn rocket() -> _ {
    rocket::build()
        .attach(CORS)
        .mount("/", routes![index])
        .mount("/", routes![user])
        .mount("/", routes![messages_poster])
        .mount("/", routes![messages_getter])
        .mount("/", routes![messages_stream])
}

#[cfg(test)]
mod test {
    use crate::{rocket_uri_macro_index, rocket_uri_macro_user};

    use super::rocket;
    use mongodb::bson::DateTime;
    use rocket::http::Status;
    use rocket::local::blocking::{Client, LocalResponse};
    use skyserver::ChatMessage;

    fn get_client() -> Client {
        Client::tracked(rocket()).expect("rocket instance should be valid")
    }

    fn get_sample_message() -> ChatMessage {
        ChatMessage {
            username: String::from("testuser"),
            message: String::from("Post Test Message"),
            datetime: DateTime::builder()
                .year(1983)
                .month(8)
                .day(19)
                .hour(23)
                .minute(15)
                .second(30)
                .build()
                .expect("Should build without errors"),
        }
    }

    fn post_sample_message(client: &Client) -> LocalResponse {
        client
            .post("/messages-handler")
            .json(&(get_sample_message()))
            .dispatch()
    }

    #[test]
    fn receives_hello_world() {
        let client = get_client();
        let response = client.get(rocket::uri!(index)).dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string(), Some("Hello, world!".into()))
    }

    #[test]
    fn can_get_sample_user() {
        let client = get_client();
        let sample_user_uri = rocket::uri!(user(name = "Sample User"));
        let response = client.get(sample_user_uri.to_string()).dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(
            response.into_string(),
            Some("User's name: Sample User".into())
        )
    }

    #[test]
    fn can_submit_message() {
        let client = get_client();
        let response = post_sample_message(&client);

        assert_eq!(response.status(), Status::Ok);
    }

    #[test]
    fn can_get_messages() {
        let client = get_client();
        post_sample_message(&client);
        let response = client.get("/messages-handler").dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert!(response
            .into_json::<Vec<ChatMessage>>()
            .expect("Deserializable chat message")
            .contains(&get_sample_message()));
    }
}
