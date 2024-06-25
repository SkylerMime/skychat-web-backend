use mongodb::{
    bson::{doc, DateTime},
    Client,
};
use serde::{Deserialize, Serialize};

const MONGODB_URI: &'static str = "mongodb://localhost";

// Represents a document in the collection
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub last_login: DateTime,
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.last_login == other.last_login
    }
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

#[cfg(test)]
mod test {
    use mongodb::{
        bson::{doc, DateTime},
        error::Error,
        options::ReplaceOptions,
        results::UpdateResult,
        Client,
    };

    use crate::{User, MONGODB_URI};

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

    #[tokio::test]
    async fn gets_sample_user() {
        upsert_sample_user()
            .await
            .expect("Sample user should be successfully upserted");
        assert_eq!(crate::get_user("Sample User").await, get_test_user());
    }
}
