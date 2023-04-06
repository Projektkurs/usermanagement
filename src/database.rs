//! database.rs - Rocket hook for the MongoDB database
//!
//! Copyright 2023 by Ben Mattes Krusekamp <ben.krause05@gmail.com>
//!
//! all items that are in the database can be addressed either by name or
//! by id with [DatabaseConnection::getfromdb_name] or [DatabaseConnection::getfromdb_id] respectivly

use crate::debug_println;
use crate::room::Room;
use crate::user::User;
use bson::oid::ObjectId;
use mongodb::{bson::doc, options::IndexOptions, IndexModel};
use rocket::{fairing, Build, Rocket};
pub use rocket_db_pools::Connection;
use rocket_db_pools::{mongodb, Database};

/// a default [Database] created using the database macro
#[derive(Database)]
#[database("main_db")]
pub struct MainDatabase(mongodb::Client);

/// trait which is only implemented by [Client][mongodb::Client] and provides get functions
/// for the [mongodb::Database] and the different collections
pub trait DatabaseUtils {
    fn db(&self) -> mongodb::Database;

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
}

impl DatabaseUtils for mongodb::Client {
    fn db(&self) -> mongodb::Database {
        self.database("helper:Paper")
    }
}

/// used by Structs which can be inserted into [MainDatabase]
pub trait DatabaseConnection
where
    Self: serde::Serialize + Sized + serde::de::DeserializeOwned + Unpin + Sync + std::marker::Send,
{
    /// get function for ObjectId ($_id)
    fn id(&self) -> ObjectId;

    /// returns the collection of the struct
    /// Self::collection is the same as db.<Self>_collection
    fn collection(db: &mongodb::Client) -> mongodb::Collection<Self>;

    /// get function for the Name, which is indexed
    fn name(&self) -> &str;

    /// the variable name which stores the name that is indexed
    /// e.g. "username" for [User] or "name" for [Room]
    fn index_name() -> &'static str;

    /// updates the content of the Database
    /// If the object is not updated, it will throw the [Error::NoUpdate] Error
    /// as it updates the whole document, it is not efficient to use this function
    /// TODO: implement an update-specific function
    async fn update(&mut self, db: &mongodb::Client) -> Result<(), Error> {
        let filter = doc! {"_id": self.id()};
        let update_doc = doc! {"$set": bson::to_document(self)?};
        let update_result = Self::collection(&db)
            .update_one(filter, update_doc, None)
            .await?;
        if update_result.modified_count > 0 {
            Ok(())
        } else {
            Err(Error::NoUpdate)
        }
    }
    /// Inserts [self] into the Database.
    /// If there is already an object with the name in the Database, it throws the
    /// [Error::AlreadyInDB] Error
    async fn insert(&self, db: &mongodb::Client) -> Result<(), Error> {
        if self.isindb(db).await {
            return Err(Error::AlreadyInDB);
        }
        Self::collection(&db).insert_one(self, None).await?;
        Ok(())
    }
    /// retrieves the item with given id from the Database.
    /// returns [Error::NotFound] if the item is not in the Database.
    /// Note that some structs have own get methods, as [User] with login.
    async fn getfromdb_id(id: &ObjectId, db: &mongodb::Client) -> Result<Self, Error> {
        if let Some(_self) = Self::collection(&db)
            .find_one(doc! {"_id": id}, None)
            .await?
        {
            return Ok(_self);
        } else {
            return Err(Error::NotFound);
        }
    }
    /// retrieves the item with given name from the Database.
    /// returns [Error::NotFound] if the item is not in the Database.
    /// Note that some structs have own get methods, as [User] with login.
    async fn getfromdb_name(name: &str, db: &mongodb::Client) -> Result<Self, Error> {
        if let Some(_self) = Self::collection(&db)
            .find_one(doc! {Self::index_name(): name}, None)
            .await?
        {
            return Ok(_self);
        } else {
            return Err(Error::NotFound);
        }
    }
    /// Returns a cursor to all items found in the Database.
    async fn get_all_from_db(db: &mongodb::Client) -> Option<mongodb::Cursor<Self>> {
        Self::collection(&db).find(None, None).await.ok()
    }
    /// looks up if the item is already in the database.
    /// It is considered to be in the database if:
    ///     1. an item with the name is in the database
    ///     2. an item with the id is in the database
    async fn isindb(&self, db: &mongodb::Client) -> bool {
        if Self::getfromdb_name(&self.name(), &db).await.is_ok() {
            return true;
        }
        if Self::getfromdb_id(&self.id(), &db).await.is_ok() {
            return true;
        }
        false
    }
}

/// a new enum of all errors that can happen while interacting with the database
/// [Error::NoUpdate] is also seen as an Error, as calling the update method on e.g. [crate::user::User]
/// should always be done after insert.
#[derive(Debug)]
pub enum Error {
    MongoDB(mongodb::error::Error),
    Bson(bson::ser::Error),
    NoUpdate,
    NotFound,
    AlreadyInDB,
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

/// takes in the [mongodb::Collection] as an argument,
/// and the field name to create an index.
/// note that this does not override any existing index,
/// meaning that this will not fix broken indices.
/// For that, you need to delete the broken index
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
    };
}
/// A fairing creates the indices that map the name field of every Struct in the database.
pub async fn create_indices(rocket: Rocket<Build>) -> fairing::Result {
    debug_println!("building indices");
    if let Some(db) = MainDatabase::fetch(&rocket) {
        if create_unique_index!(db.0.user_collection(), "username").is_err()
            || create_unique_index!(db.0.room_collection(), "name").is_err()
            || create_unique_index!(db.0.layout_collection(), "name").is_err()
            || create_unique_index!(db.0.epaper_collection(), "name").is_err()
        {
            return Err(rocket);
        }
        return Ok(rocket);
    } else {
        Err(rocket)
    }
}
