use futures::TryStreamExt;
use mongodb::{
    bson::{self, doc, DateTime},
    error::Error,
    results::InsertOneResult,
    Client, Cursor,
};
use serde::{Deserialize, Serialize};

const MONGODB_URI: &'static str = "mongodb://localhost";

// TODO: Replace Expects with Match

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
    // serializes as an RFC 3339 / ISO-8601 string, which is how the Frontend will send it as JSON
    #[serde(with = "bson::serde_helpers::bson_datetime_as_rfc3339_string")]
    pub datetime: DateTime,
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

async fn extract_messages_from_cursor(
    mut messages_cursor: Cursor<ChatMessage>,
) -> Vec<ChatMessage> {
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

pub async fn get_messages() -> Vec<ChatMessage> {
    let messages_collection = get_messages_collection().await;
    let messages_cursor = messages_collection
        .find(doc! {}, None)
        .await
        .expect("Find should find messages");

    extract_messages_from_cursor(messages_cursor).await
}

pub async fn get_all_messages_after_date(min_date: DateTime) -> Vec<ChatMessage> {
    let messages_collection = get_messages_collection().await;
    // Note: MongoDB uses the same Document object for queries.
    let min_date = min_date
        .try_to_rfc3339_string()
        .expect("Date should be serializable");
    let date_filter = doc! {
        "datetime": { "$gt": min_date }
    };
    let messages_cursor = messages_collection
        .find(date_filter, None)
        .await
        .expect("Find should find messages");

    extract_messages_from_cursor(messages_cursor).await
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
            datetime: DateTime::builder()
                .year(1975)
                .month(8)
                .day(19)
                .hour(23)
                .minute(15)
                .second(30)
                .build()
                .expect("Should build without errors"),
        };
        crate::put_message(test_message.clone())
            .await
            .expect("No post errors should occur in test");
        assert!(crate::get_messages().await.contains(&test_message));
    }

    #[tokio::test]
    async fn only_gets_later_messages() {
        delete_all_testuser_messages()
            .await
            .expect("Sample messages should be cleared before test");
        let early_message = ChatMessage {
            username: String::from("testuser"),
            message: String::from("Early Message"),
            datetime: DateTime::builder()
                .year(2005)
                .month(1)
                .day(1)
                .build()
                .expect("Should build without errors"),
        };
        let late_message = ChatMessage {
            username: String::from("testuser"),
            message: String::from("Late Message"),
            datetime: DateTime::builder()
                .year(2105)
                .month(1)
                .day(1)
                .build()
                .expect("Should build without errors"),
        };
        crate::put_message(early_message.clone())
            .await
            .expect("No post errors should occur in test");
        crate::put_message(late_message.clone())
            .await
            .expect("No post errors should occur in test");
        let min_date_filter = DateTime::builder()
            .year(2100)
            .month(1)
            .day(1)
            .build()
            .expect("Should build without errors");
        let retrieved_messages = crate::get_all_messages_after_date(min_date_filter).await;
        delete_all_testuser_messages()
            .await
            .expect("Sample messages should be cleared before test");
        assert_eq!(retrieved_messages.len(), 1);
        assert_eq!(retrieved_messages[0], late_message);
    }
}
