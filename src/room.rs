use crate::{
    database::{self, DatabaseUtils},
    event::{self, Event},
    user, MainDatabase,
};
use bson::doc;
use bson::oid::ObjectId;

use chrono::{DateTime, Local, TimeZone, Timelike};

use rocket::form::Form;
use rocket::Route;
use rocket_db_pools::Connection;
use serde::*;

use std::collections::HashMap;
use std::collections::LinkedList;

#[derive(Default, Debug, Serialize, Deserialize)]
//todo: use a BTreeMap instead of a LinkedList or a BTreeSet
pub struct Room {
    #[serde(rename = "_id")]
    id: ObjectId,
    name: String,
    //TODO: use btreemap instead
    events: LinkedList<Event>,
    layouts: Vec<ObjectId>,
    layout_values: HashMap<String, String>,
    owner: Option<ObjectId>,
    description: Option<String>,
}
impl Room {
    /// creates a new Room with a new Object id.
    /// owner and description are left empty
    pub fn create(name: String) -> Self {
        Room {
            id: ObjectId::new(),
            name,
            events: LinkedList::new(),
            layouts: Vec::new(),
            layout_values: HashMap::new(),
            owner: None,
            description: None,
        }
    }

    /// looks up, wether [Room] could accomodate the given event
    /// accomodate means in this context that there is no overlaping event in
    /// the time slot. The event can parital overlap with another event, e.g. one event can end at 12:00 while the other starts at 12:00
    pub fn could_accomodate(&self, event: Event) -> bool {
        for currentevent in &self.events {
            if !(currentevent.start().timestamp() >= event.stop().timestamp()
                || currentevent.stop().timestamp() <= event.start().timestamp())
            {
                return false;
            }
            if currentevent.stop().timestamp() <= event.start().timestamp() {
                return true;
            }
        }
        true
    }
    pub fn add_event(&mut self, event: Event) -> Option<()> {
        let mut cursor = self.events.cursor_front_mut();
        if let Some(currentevent) = cursor.current() {
            if event.stop().timestamp() <= currentevent.start().timestamp() {
                cursor.insert_before(event);
                return Some(());
            }
        }
        let mut greater_than_before = false;
        loop {
            
            if let Some(currentevent) = cursor.current() {
                if event.overlaps_with(&currentevent){
                    println!("overlaps");
                    return None;
                }
                if greater_than_before {
                    if event.stop().timestamp() <= currentevent.start().timestamp() {
                            cursor.insert_before(event);
                            return Some(());
                    }
                }
                if event.start().timestamp() >= currentevent.stop().timestamp() {
                    greater_than_before = true;
                }// else {
                //    cursor.insert_after(event);
                //    return Some(());
                //}
                //println!("{:?}", currentevent);

                cursor.move_next();
            } else {
                //return None;
                self.events.push_back(event);
                return Some(());
            }
        }
    }
    /// returns true if successfull
    pub fn remove_event_id(&mut self, event_id: bson::oid::ObjectId) -> bool {
        let cursor = &mut self.events.cursor_front_mut();
        loop {
            if let Some(currentevent) = &cursor.current() {
                if currentevent.id() == event_id {
                    cursor.remove_current();
                    return true;
                }
            } else {
                return false;
            }
            cursor.move_next();
        }
    }
    pub async fn remove_event_datetime(&mut self, datetime: DateTime<Local>,db: &mongodb::Client) -> Option<()> {
        let mut cursor = self.events.cursor_front_mut();
        loop {
            if let Some(currentevent) = &cursor.current() {
                if currentevent.datetime_is_in(&datetime){
                    println!("found item");
                    cursor.remove_current()?;
                    self.update(&db).await.ok()?;
                    return Some(())
                }
            } else {
                return None;
            }
            cursor.move_next();
        }

    }
    async fn update(&mut self, db: &mongodb::Client) -> Result<(), database::Error> {
        let filter = doc! {"_id": self.id};
        let update_doc = doc! {"$set": bson::to_document(self)?};
        let update_result = db
            .room_collection()
            .update_one(filter, update_doc, None)
            .await?;
        if update_result.modified_count > 0 {
            Ok(())
        } else {
            Err(database::Error::NoUpdate)
        }
    }
    //todo: finish
    async fn get_event_range(&self, start: DateTime<Local>, stop: DateTime<Local>) -> Vec<&Event> {
        let mut ret = Vec::new();
        for event in &self.events {
            if event.start().timestamp() >= start.timestamp()
                && event.stop().timestamp() <= stop.timestamp()
            {
                ret.push(event);
            }
            if event.stop().timestamp() > stop.timestamp() {
                break;
            }
        }
        ret
    }
    async fn insert(&self, db: &mongodb::Client) -> Option<()> {
        if self.isindb(db).await {
            return None;
        }
        db.room_collection().insert_one(self, None).await.ok()?;
        Some(())
    }
    pub async fn getfromdb_name(name: &str, db: &mongodb::Client) -> Option<Room> {
        db.room_collection()
            .find_one(doc! {"name": name}, None)
            .await
            .ok()?
    }
    pub async fn get_all_from_db(db: &mongodb::Client) -> Option<mongodb::Cursor<Room>> {
        db.room_collection().find(None, None).await.ok()
    }
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub async fn getfromdb_id(id: &ObjectId, db: &mongodb::Client) -> Option<Room> {
        db.room_collection()
            .find_one(doc! {"_id": id}, None)
            .await
            .ok()?
    }
    async fn isindb(&self, db: &mongodb::Client) -> bool {
        if let Some(_room) = Room::getfromdb_id(&self.id, &db).await {
            return true;
        }
        false
    }
}
#[derive(Debug, FromForm)]
struct CreateDeleteForm<'r> {
    userdata: user::UserData<'r>,
    name: &'r str,
}
#[post("/create", data = "<form>")]
async fn create(form: Form<CreateDeleteForm<'_>>, db: Connection<MainDatabase>) -> Option<()> {
    println!("get user");
    let user = user::User::login(
        form.userdata.username,
        String::from(form.userdata.password),
        &*db,
    )
    .await
    .ok()?;
    println!("got user");
    if !user.can_create_rooms() || Room::getfromdb_name(form.name, &db).await.is_some() {
        println!("user cannot create rooms");
        return None;
    }
    let room = Room::create(String::from(form.name));
    println!("insert user");
    room.insert(&db).await?;
    Some(())
}
#[post("/delete", data = "<form>")]
async fn delete(form: Form<CreateDeleteForm<'_>>, db: Connection<MainDatabase>) -> Option<()> {
    println!("get user");
    let user = user::User::login(
        form.userdata.username,
        String::from(form.userdata.password),
        &*db,
    )
    .await
    .ok()?;
    println!("got user");
    let room = Room::getfromdb_name(form.name, &db).await?;
    if !user.can_edit_room(&room.id) {
        println!("cannot delete rooms");
        return None;
    }
    db.room_collection()
        .delete_one(doc! {"_id": room.id}, None)
        .await
        .ok()?;
    println!("insert user");
    Some(())
}
#[derive(Debug, FromForm)]
struct CreateEventForm<'a> {
    userdata: user::UserData<'a>,
    room_name: Option<String>,
    room_id: Option<String>,
    headline: String,
    description: Option<String>,
    start: String,
    stop: String,
}

#[post("/get?<name>", data = "<form>")]
async fn get(
    name: &str,
    form: Form<user::UserData<'_>>,
    db: Connection<MainDatabase>,
) -> Option<String> {
    println!("room:{}", name);
    let user = &user::User::login(form.username, String::from(form.password), &*db)
        .await
        .ok()?;
    println!("search for room");
    let room = Room::getfromdb_name(name, &*db).await?;
    if !user.can_edit_room(&room.id) {
        println!("cannot edit rooms");
        return None;
    }
    Some(rocket::serde::json::to_string(&room).ok()?)
}

#[post("/add_event", data = "<form>")]
async fn add_event(form: Form<CreateEventForm<'_>>, db: Connection<MainDatabase>) -> Option<()> {
    let user = user::User::login(
        form.userdata.username,
        String::from(form.userdata.password),
        &*db,
    )
    .await
    .ok()?;
    println!("1");
    let mut room = if let Some(room_id) = &form.room_id {
        let id = rocket::serde::json::from_str::<ObjectId>(&room_id).ok()?;
        Room::getfromdb_id(&id, &db).await?
    } else {
        Room::getfromdb_name(&form.room_name.clone()?, &db).await?
    };
    println!("2:{}", &form.stop);
    let start = DateTime::parse_from_rfc3339(&form.start).ok()?.into();
    let stop = DateTime::parse_from_rfc3339(&form.stop).ok()?.into();
    println!("2.5");
    //let stop = rocket::serde::json::from_str::<DateTime<Local>>(&form.stop).ok()?;
    //let start = rocket::serde::json::from_str::<DateTime<Local>>(&form.start).ok()?;

    println!("3");
    // way to do this without cloning?
    let event = event::Event::create(
        user.id().to_string(),
        form.headline.clone(),
        form.description.clone(),
        start,
        stop,
    )?;
    println!("4");
    if !user.can_edit_room(&room.id) {
        return None;
    }
    println!("5");
    room.add_event(event)?;
    println!("6");
    room.update(&db).await.ok()?;
    Some(())
}
#[derive(Debug, FromForm)]
struct RemoveEventForm<'r> {
    userdata: user::UserData<'r>,
    room_name: Option<String>,
    room_id: Option<String>,
    remove_datetime: String
}
#[post("/remove_event", data = "<form>")]
async fn remove_event(form: Form<RemoveEventForm<'_>>, db: Connection<MainDatabase>) -> Option<()> {
    let _user = user::User::login(
        form.userdata.username,
        String::from(form.userdata.password),
        &*db,
    )
    .await
    .ok()?;
let mut room = if let Some(room_id) = &form.room_id {
    let id = rocket::serde::json::from_str::<ObjectId>(&room_id).ok()?;
    Room::getfromdb_id(&id, &db).await?
} else {
    Room::getfromdb_name(&form.room_name.clone()?, &db).await?
};
    let datetime = DateTime::parse_from_rfc3339(&form.remove_datetime).ok()?.into();
    println!("remove event");
    room.remove_event_datetime(datetime, &db).await?;
    //room.update(&db).await.ok()?;
    Some(())
}


#[derive(Debug, FromForm)]
struct GetEventRangeForm<'a> {
    userdata: user::UserData<'a>,
    room_name: Option<String>,
    room_id: Option<String>,
    start: Option<String>,
    stop: Option<String>,
    get_day_from_sec_since_utc: Option<i64>,
    get_current_day: Option<bool>,
    get_current_day_offset_in_days: Option<i32>,
}
//todo: check wether user has access to room
//todo: use match statement to reduce boilerplate
#[post("/get_event_range", data = "<form>")]
async fn get_event_range(
    form: Form<GetEventRangeForm<'_>>,
    db: Connection<MainDatabase>,
) -> Option<String> {
    let _user = user::User::login(
        form.userdata.username,
        String::from(form.userdata.password),
        &*db,
    )
    .await
    .ok()?;
    let room = if let Some(room_id) = &form.room_id {
        let id = rocket::serde::json::from_str::<ObjectId>(&room_id).ok()?;
        Room::getfromdb_id(&id, &db).await?
    } else {
        Room::getfromdb_name(&form.room_name.clone()?, &db).await?
    };
    //if let syntax would be used, the Strings would be needed to be cloned
    if let Some(form_start)=&form.start && let Some(form_stop)=&form.stop {
        //todo: does not work currently
    let start = rocket::serde::json::from_str::<DateTime<Local>>(form_start).ok()?;
    //let start = DateTime::try_from(start.timestamp()-start.num_seconds_from_midnight()).ok()?;
    let stop = rocket::serde::json::from_str::<DateTime<Local>>(form_stop).ok()?;
    //let stop = DateTime::try_from(stop.timestamp()-stop.num_seconds_from_midnight()).ok()?;
    let events = room.get_event_range(start, stop).await;
    return rocket::serde::json::to_string(&events).ok()
    }

    if let Some(timestamp) = form.get_day_from_sec_since_utc {
        let start = Local.timestamp_opt(timestamp, 0).earliest()?;
        let stop = start.checked_add_days(chrono::Days::new(1))?;
        //let stop = start.offset()

        let events = room.get_event_range(start, stop).await;
        return rocket::serde::json::to_string(&events).ok();
    }

    if form.get_current_day == Some(true) {
        let mut start = Local::now();
        //let duration = chrono::Duration::seconds(start.num_seconds_from_midnight() as i64);
        //println!{"duration:{}",duration};
        //start = start - duration;
        start = start.with_hour(0)?.with_minute(0)?.with_second(0)?.with_nanosecond(0)?;
        if let Some(days) = form.get_current_day_offset_in_days {
            if days >= 0 {
                start = start.checked_add_days(chrono::Days::new(days as u64))?;
            } else {
                start = start.checked_sub_days(chrono::Days::new((days * -1) as u64))?;
            }
        }
        let stop = start.clone().checked_add_days(chrono::Days::new(1))?;

        let events = room.get_event_range(start, stop).await;
        return rocket::serde::json::to_string(&events).ok();
    }
    return None;
}

pub fn routes() -> Vec<Route> {
    routes![create, delete, get, add_event,remove_event, get_event_range]
}
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, Timelike};
    use mongodb::Client;
    #[tokio::test]
    async fn event_creation() {
        let db = Client::with_uri_str("mongodb://localhost:27017")
            .await
            .expect("Connection to mongodb could not be established");
        let room_opt = Room::getfromdb_name("test-room", &db).await;
        let mut room = if let Some(room) = room_opt {
            room
        } else {
            println!("room not found");
            let room = Room::create(String::from("test-room"));
            room.insert(&db).await.expect("insertion failed");
            room
        };
        //let start= Local.timestamp_nanos(0);
        let start = Local::now();
        let stop = start
            .with_minute(start.minute() + 1)
            .expect("failed just because it is HH:59, just rerun the test later");
        room.add_event(
            Event::create(
                String::from("fake-id"),
                String::from("headline"),
                None,
                start,
                stop,
            )
            .expect("could not generate event"),
        )
        .expect("failed to add event. Note this test can just run once every minute as it adds an Event that lasts one minute");
        room.update(&db).await.expect("room could not be updated");
    }
}
