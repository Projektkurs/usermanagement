use crate::{
    database::{self, DatabaseUtils},
    debug_println,
    event::{self, Event},
    user, MainDatabase,
};
use bson::doc;
use bson::oid::ObjectId;

use crate::database::DatabaseConnection;
use chrono::{DateTime, Local, TimeZone, Timelike};
use rocket::form::Form;
use rocket::Route;
use rocket_db_pools::Connection;
use serde::*;
use std::collections::BTreeSet;

use std::collections::HashMap;

#[derive(Default, Debug, Serialize, Deserialize)]
//todo: use a BTreeMap instead of a LinkedList or a BTreeSet
pub struct Room {
    #[serde(rename = "_id")]
    id: ObjectId,
    name: String,
    //TODO: use btreemap instead
    //events: LinkedList<Event>,
    events: BTreeSet<Event>,
    layouts: Vec<ObjectId>,
    layout_values: HashMap<String, String>,
    owner: Option<ObjectId>,
    description: Option<String>,
}
impl database::DatabaseConnection for Room {
    #[inline]
    fn id(&self) -> ObjectId {
        self.id
    }
    #[inline]
    fn collection(db: &mongodb::Client) -> mongodb::Collection<Self> {
        db.room_collection()
    }
    #[inline]
    fn name(&self) -> &str {
        &self.name
    }
    #[inline]
    fn index_name() -> &'static str {
        "name"
    }
}
impl Room {
    /// creates a new Room with a new Object id.
    /// owner and description are left empty
    pub fn create(name: String) -> Self {
        Room {
            id: ObjectId::new(),
            name,
            events: BTreeSet::new(),
            layouts: Vec::new(),
            layout_values: HashMap::new(),
            owner: None,
            description: None,
        }
    }

    /// looks up, wether [Room] could accomodate the given event
    /// accomodate means in this context that there is no overlaping event in
    /// the time slot. The event can parital overlap with another event, e.g. one event can end at 12:00 while the other starts at 12:00
    pub fn could_accomodate(&self, event: &Event) -> bool {
        if event.isdummy() {
            return false;
        }
        for currentevent in &self.events {
            if !(currentevent.start() >= event.stop() || currentevent.stop() <= event.start()) {
                return false;
            }
            if currentevent.stop() <= event.start() {
                return true;
            }
        }
        true
    }
    /// add an event to the room. returns [None] if the Event is already present or
    /// it would overlap with any event. Panics if the event is a dummy
    pub fn add_event(&mut self, event: Event) -> Option<()> {
        if event.isdummy() {
            // as this should never happen, it not just returns None
            panic!("tried to add an event that is a dummy")
        }
        if !self.could_accomodate(&event) {
            return None;
        }

        if self.events.insert(event) {
            return Some(());
        }
        None
    }
    /// Removes the Event of the room if
    /// 1. The [DateTime] overlaps with an Event
    /// 2. The start of an Event is the [DateTime]
    pub fn remove_event_datetime(&mut self, datetime: DateTime<Local>) -> Option<()> {
        let event = Event::create_dummy(datetime);
        if self.events.remove(&event) {
            return Some(());
        }
        let smaller = self.events.range(..&event).next_back()?.clone();
        if event.overlaps_with(&smaller) {
            self.events.remove(&smaller);
            return Some(());
        }
        debug_println!("room not found");
        None
    }

    async fn get_event_range(&self, start: DateTime<Local>, stop: DateTime<Local>) -> Vec<&Event> {
        let mut ret = Vec::new();

        for event in self
            .events
            .range(Event::create_dummy(start)..Event::create_dummy(stop))
            .into_iter()
        {
            ret.push(event)
        }
        ret
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
    if !user.can_create_rooms() || Room::getfromdb_name(form.name, &db).await.is_ok() {
        println!("user cannot create rooms");
        return None;
    }
    let room = Room::create(String::from(form.name));
    println!("insert user");
    room.insert(&db).await.ok()?;
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
    let room = Room::getfromdb_name(form.name, &db).await.ok()?;
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
    let room = Room::getfromdb_name(name, &*db).await.ok()?;
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
        Room::getfromdb_id(&id, &db).await.ok()?
    } else {
        Room::getfromdb_name(&form.room_name.clone()?, &db)
            .await
            .ok()?
    };
    let start = DateTime::parse_from_rfc3339(&form.start).ok()?.into();
    let stop = DateTime::parse_from_rfc3339(&form.stop).ok()?.into();

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
    remove_datetime: String,
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
        Room::getfromdb_id(&id, &db).await.ok()?
    } else {
        Room::getfromdb_name(&form.room_name.clone()?, &db)
            .await
            .ok()?
    };
    let datetime = DateTime::parse_from_rfc3339(&form.remove_datetime)
        .ok()?
        .into();
    println!("remove event");
    room.remove_event_datetime(datetime)?;
    room.update(&db).await.ok()?;
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
//todo: check whether the user has access to the room
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
        Room::getfromdb_id(&id, &db).await.ok()?
    } else {
        Room::getfromdb_name(&form.room_name.clone()?, &db)
            .await
            .ok()?
    };
    //if let syntax would be used, the Strings would be needed to be cloned
    if let Some(form_start)=&form.start && let Some(form_stop)=&form.stop {
        //todo: does not work currently
    let start = rocket::serde::json::from_str::<DateTime<Local>>(form_start).ok()?;

    let stop = rocket::serde::json::from_str::<DateTime<Local>>(form_stop).ok()?;

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
        start = start
            .with_hour(0)?
            .with_minute(0)?
            .with_second(0)?
            .with_nanosecond(0)?;
        if let Some(days) = form.get_current_day_offset_in_days {
            if days >= 0 {
                start = start.checked_add_days(chrono::Days::new(days as u64))?;
            } else {
                start = start.checked_sub_days(chrono::Days::new((days * -1) as u64))?;
            }
        }
        println!("startdate:{:?}", start);
        let mut stop = start.clone().checked_add_days(chrono::Days::new(1))?;
        //todo: send bug report to chrono, as the +2 is a bug with crono. checked_add_days disregards the Local time zone, leading to a -2 hour gap.
        stop = stop.with_hour(stop.hour() + 2)?;
        println!("stoptdate:{:?}", stop);
        let events = room.get_event_range(start, stop).await;
        return rocket::serde::json::to_string(&events).ok();
    }
    return None;
}

pub fn routes() -> Vec<Route> {
    routes![
        create,
        delete,
        get,
        add_event,
        remove_event,
        get_event_range
    ]
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
        let mut room = if let Ok(room) = room_opt {
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
