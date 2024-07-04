use futures::TryStreamExt;
use mongodb::{
    bson::{doc, DateTime},
    error::Error,
    results::InsertOneResult,
    Client,
};
use serde::{Deserialize, Serialize};

const MONGODB_URI: &'static str = "mongodb://localhost";

// Represents a document in the Users collection
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub name: String,
    pub last_login: DateTime,
}

// Represents a document in the Chat Messages collection
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ChatMessage {
    pub username: String,
    pub message: String,
}

// Database functions
pub async fn get_user(name: &str) -> User {
    let client = Client::with_uri_str(MONGODB_URI)
        .await
        .expect("Database should be connectable");

    // Get user bios collection
    let users_collection = client.database("skyserver").collection::<User>("users");

    let retrieved_user: User = users_collection
        .find_one(doc! { "name": name}, None)
        .await
        .expect("User should be in the database")
        .unwrap();

    retrieved_user
}

pub async fn get_messages_collection() -> mongodb::Collection<ChatMessage> {
    let client = Client::with_uri_str(MONGODB_URI)
        .await
        .expect("Database should be connectable");

    let db = client.database("skyserver");
    db.collection::<ChatMessage>("messages")
}

pub async fn put_message(message: ChatMessage) -> Result<InsertOneResult, Error> {
    let messages_collection = get_messages_collection().await;
    messages_collection.insert_one(message, None).await
}

pub async fn get_messages() -> Vec<ChatMessage> {
    let messages_collection = get_messages_collection().await;
    let mut messages_cursor = messages_collection
        .find(doc! {}, None)
        .await
        .expect("Find should find messages");

    let mut chat_messages: Vec<ChatMessage> = Vec::new();
    while let Some(message) = messages_cursor
        .try_next()
        .await
        .expect("Should get the next thing")
    {
        chat_messages.push(message);
    }
    chat_messages
}

#[cfg(test)]
mod test {
    use mongodb::{
        bson::{doc, DateTime},
        error::Error,
        options::ReplaceOptions,
        results::{DeleteResult, UpdateResult},
        Client,
    };

    use crate::{get_messages_collection, ChatMessage, User, MONGODB_URI};

    fn get_test_user() -> User {
        User {
            name: String::from("Sample User"),
            last_login: DateTime::from_millis(0),
        }
    }

    async fn upsert_sample_user() -> Result<UpdateResult, Error> {
        let client = Client::with_uri_str(MONGODB_URI)
            .await
            .expect("Database should be connectable");

        // Get user bios collection
        let database = client.database("skyserver");
        let users_collection = database.collection::<User>("users");

        let replace_user = get_test_user();
        let update_options: ReplaceOptions = ReplaceOptions::builder().upsert(true).build();

        users_collection
            .replace_one(doc! { "name": "Sample User" }, replace_user, update_options)
            .await
    }

    async fn delete_all_testuser_messages() -> Result<DeleteResult, Error> {
        let chat_collection = get_messages_collection().await;

        chat_collection
            .delete_many(doc! { "username": "testuser" }, None)
            .await
    }

    #[tokio::test]
    async fn gets_sample_user() {
        upsert_sample_user()
            .await
            .expect("Sample user should be successfully upserted");
        assert_eq!(crate::get_user("Sample User").await, get_test_user());
    }

    #[tokio::test]
    async fn puts_and_gets_messages() {
        delete_all_testuser_messages()
            .await
            .expect("Sample messages should be cleared before test");
        let test_message = ChatMessage {
            username: String::from("testuser"),
            message: String::from("Test Message"),
        };
        crate::put_message(test_message.clone())
            .await
            .expect("No post errors in test");
        assert!(crate::get_messages().await.contains(&test_message));
    }
}
