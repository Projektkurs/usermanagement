use std::collections::HashMap;

use crate::user;
use crate::MainDatabase;
use bson::oid::ObjectId;
use rocket::form::Form;
//use crate::room::Room;
use rocket_db_pools::Connection;
//use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Epaper {
    #[serde(rename = "_id")]
    id: ObjectId,
    name: String,
    ip: String,
    //activelayout_layout: ObjectId,
    //update_rate: DateTime<Local>,
    //room: Option<ObjectId>,
}

#[post("/get", data = "<form>")]
async fn get(form: Form<user::UserData<'_>>, db: Connection<MainDatabase>) -> Option<String> {
    let _user = &user::User::login(form.username, String::from(form.password), &*db)
        .await
        .ok()?;
    println!("user");
    let mut map = HashMap::new();
    map.insert("neues Epaper", "1.2.3.4");
    map.insert("altes Epaper", "2.3.4.5");
    println!("output");
    Some(rocket::serde::json::to_string(&map).ok()?)
}
pub fn routes() -> Vec<rocket::Route> {
    routes![get]
}
