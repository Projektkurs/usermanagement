//! main.rs - A Rocket backend for a Room Reservation
//! 
//! This backend is based on a REST API style whitch uses x-form
//! to validate the identity of the request
//!
//! Copyright 2023 by Ben Mattes Krusekamp <ben.krause05@gmail.com>
#![feature(let_chains)]
#![allow(dead_code)]
#![feature(async_fn_in_trait)]

#[macro_use]
extern crate rocket;

mod database;
mod epaper;
mod event;
mod image;
mod room;
mod user;

use rocket::fairing::AdHoc;
use rocket_db_pools::Database;
use database::MainDatabase;


#[launch]
fn rocket() -> _ {
    println!("{:?}", user::routes()[1].uri);
    rocket::build()
        .attach(database::MainDatabase::init())
        .attach(AdHoc::try_on_ignite(
            "Create collection indices",
            database::create_indices,
        ))
        .register("/", catchers![not_found])
        .mount("/", routes![status])
        .mount("/epaper", epaper::routes())
        .mount("/user", user::routes())
        .mount("/room", room::routes())
        .mount("/image", image::routes())
}

/// used to look up whether the given IP is a server
/// or if the server is online.
#[get("/status")]
async fn status() -> Option<&'static str> {
    Some("ok")
}

/// As it is an API, it can just send an empty str
/// if the request fails.
#[catch(404)]
fn not_found() -> &'static str {
    ""
}


/// just a wrapper around println! which looks up, wether it is in DEBUG mode
#[macro_export]
macro_rules! debug_println {
    ($($rest:tt)*) => {
        if std::env::var("DEBUG").is_ok() {
            std::println!($($rest)*);
        }
    }
}
