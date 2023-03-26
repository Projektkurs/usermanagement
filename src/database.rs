use crate::room::Room;
use crate::user::User;
use mongodb::bson::doc;
use mongodb::{options::IndexOptions, IndexModel};
use rocket::fairing;
use rocket::{Build, Rocket};
pub use rocket_db_pools::Connection;
use rocket_db_pools::{mongodb, Database};

#[derive(Database)]
#[database("main_db")]
pub struct MainDatabase(mongodb::Client);
#[derive(Debug)]
pub enum Error {
    MongoDB(mongodb::error::Error),
    Bson(bson::ser::Error),
    NoUpdate,
}
impl From<mongodb::error::Error> for Error {
    fn from(error: mongodb::error::Error) -> Self {
        Error::MongoDB(error)
    }
}
impl From<bson::ser::Error> for Error {
    fn from(error: bson::ser::Error) -> Self {
        Error::Bson(error)
    }
}
pub trait DatabaseUtils {
    fn db(&self) -> mongodb::Database;

    fn user_collection(&self) -> mongodb::Collection<User>;

    fn room_collection(&self) -> mongodb::Collection<Room>;

    fn epaper_collection(&self) -> mongodb::Collection<Room>;

    fn layout_collection(&self) -> mongodb::Collection<Room>;

    fn create_example_users(&self) -> Option<&Self>;
}

impl DatabaseUtils for mongodb::Client {
    fn db(&self) -> mongodb::Database {
        self.database("helper:Paper")
    }

    fn user_collection(&self) -> mongodb::Collection<User> {
        self.db().collection::<User>("users")
    }
    fn room_collection(&self) -> mongodb::Collection<Room> {
        self.db().collection::<Room>("rooms")
    }
    fn epaper_collection(&self) -> mongodb::Collection<Room> {
        self.db().collection::<Room>("epapers")
    }
    fn layout_collection(&self) -> mongodb::Collection<Room> {
        self.db().collection::<Room>("layouts")
    }
    fn create_example_users(&self) -> Option<&Self> {
        Some(&self)
    }
}

#[macro_export]
macro_rules! create_unique_index {
    ($collection:expr, $field:expr) => {
        $collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! {$field: 1})
                    .options(IndexOptions::builder().unique(true).build())
                    .build(),
                None,
            )
            .await
            .ok()
            .unwrap();
    };
}

pub async fn create_indexes(rocket: Rocket<Build>) -> fairing::Result {
    println!("setting up db");
    if let Some(db) = MainDatabase::fetch(&rocket) {
        create_unique_index!(db.0.user_collection(), "username");
        create_unique_index!(db.0.room_collection(), "name");
        create_unique_index!(db.0.epaper_collection(), "name");
        create_unique_index!(db.0.layout_collection(), "name");
        return Ok(rocket);
    } else {
        Err(rocket)
    }
}
