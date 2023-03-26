#![feature(linked_list_cursors)]
#![feature(let_chains)]
#![allow(dead_code)]

use rocket::fairing::AdHoc;
use rocket_db_pools::Database;
mod database;
mod epaper;
mod event;
mod image;
mod layout;
mod room;
mod test;
mod user;
use database::MainDatabase;
#[macro_use]
extern crate rocket;
#[launch]
fn rocket() -> _ {
    println!("{:?}", user::routes()[1].uri);
    rocket::build()
        .attach(MainDatabase::init())
        .attach(AdHoc::try_on_ignite(
            "Create collection indexes",
            database::create_indexes,
        ))
        .register("/", catchers![not_found])
        .mount("/", routes![status])
        .mount("/epaper", epaper::routes())
        .mount("/user", user::routes())
        .mount("/room", room::routes())
        .mount("/image", image::routes())
}

#[get("/status")]
async fn status() -> Option<&'static str> {
    Some("ok")
}
#[catch(404)]
fn not_found() -> String {
    String::from("")
}
