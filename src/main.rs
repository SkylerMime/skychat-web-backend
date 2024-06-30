use rocket::{self, routes};

#[rocket::get("/")]
async fn index() -> &'static str {
    "Hello, world!"
}

#[rocket::get("/user/<name>")]
async fn user(name: &str) -> String {
    let retrieved_user = skyserver::get_user(name).await;
    format!("User's name: {}", retrieved_user.name)
}

#[rocket::launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/", routes![user])
}

#[cfg(test)]
mod test {
    use crate::{rocket_uri_macro_index, rocket_uri_macro_user};

    use super::rocket;
    use rocket::http::Status;
    use rocket::local::blocking::Client;

    fn get_client() -> Client {
        Client::tracked(rocket()).expect("rocket instance should be valid")
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
}
