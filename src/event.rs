use bson::oid::ObjectId;
use chrono::Timelike;
use chrono::{DateTime, Local};
use serde::*;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Event {
    #[serde(rename = "_id")]
    id: ObjectId,
    booker_id: String,
    headline: String,
    description: Option<String>,

    start: DateTime<Local>,
    stop: DateTime<Local>,
}
impl Event {
    pub fn create(
        booker_id: String,
        headline: String,
        description: Option<String>,
        start: DateTime<Local>,
        stop: DateTime<Local>,
    ) -> Option<Self> {
        if start.timestamp() >= stop.timestamp() || booker_id == "" {
            return None;
        }
        let start = start.with_nanosecond(0)?.with_second(0)?;
        let stop = stop.with_nanosecond(0)?.with_second(0)?;
        Some(Event {
            id: ObjectId::new(),
            booker_id,
            headline,
            description,
            start,
            stop,
        })
    }
    pub fn start(&self) -> DateTime<Local> {
        self.start
    }
    pub fn stop(&self) -> DateTime<Local> {
        self.stop
    }
    pub fn id(&self) -> ObjectId {
        self.id
    }
    pub fn overlaps_with(&self,event: &Event) -> bool{
        if self.start.timestamp()>=event.start.timestamp() && self.start.timestamp()<event.stop.timestamp(){
            return true;
        }
        if self.stop.timestamp()>event.start.timestamp() && self.stop.timestamp()<=event.stop.timestamp(){
            return true;
        }
        return false;
    }
    pub fn datetime_is_in(&self,datetime: &DateTime<Local>) -> bool{
        if datetime.timestamp() >=self.start.timestamp() && datetime.timestamp()<=self.stop.timestamp(){
            return true;
        }
        return false;
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Days, Timelike};
    fn defaultevent(start: DateTime<Local>, stop: Option<DateTime<Local>>) -> Event {
        if let Some(stop) = stop {
            Event::create(
                String::from("test"),
                String::from("test event"),
                None,
                start,
                stop,
            )
            .expect("event creation with stop time failed")
        } else {
            Event::create(
                String::from("test"),
                String::from("test event"),
                None,
                start,
                start
                    .with_minute((start.minute() + 1) % 60)
                    .expect("creating stop time failed"),
            )
            .expect("event creation failed")
        }
    }
    use super::*;
    use crate::room::Room;
    #[test]
    fn test_add_event_on_empty() {
        let mut room = Room::default();
        let event = defaultevent(Local::now(), None);
        (room.add_event(event)).unwrap()
    }
    #[test]
    #[should_panic]
    fn test_add_event_overlap() {
        let mut room = Room::default();
        let now = Local::now();
        let mut event = Event::create(
            String::from("test"),
            String::from("test event"),
            None,
            now,
            now,
        )
        .unwrap();
        event.start = event.start.checked_sub_days(Days::new(1)).unwrap();
        event.stop = event.stop.checked_add_days(Days::new(1)).unwrap();
        println!("first event:{:?}", event);
        room.add_event(event.clone()).unwrap();
        //let event = Event::default();
        println!("second event:{:?}", event);
        room.add_event(event).unwrap();
    }
    #[test]
    /// As one event could start at 12 while another stops at 12, partial overlap is allowed.
    /// todo: Sommer und Winterzeit
    fn test_add_event_partial_overlap() {
        let mut room = Room::default();
        let now = Local::now();
        let mut event = defaultevent(now, None);
        event.stop = event.start; //subtract the one minute of defaultevent
        event.start = event.start.checked_sub_days(Days::new(1)).unwrap();
        println!("first event:{:?}", event);
        room.add_event(event).unwrap();
        let mut event = defaultevent(now, None);
        event.stop = event.stop.checked_add_days(Days::new(1)).unwrap();
        println!("second event:{:?}", event);
        room.add_event(event).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_remove_event_on_empty() {
        let mut room = Room::default();
        assert!(room.remove_event(ObjectId::new()));
    }
    #[test]
    #[should_panic]
    fn test_remove_event_unknown_id() {
        let mut room = Room::default();
        let now = Local::now();
        let event = defaultevent(now, None);
        room.add_event(event.clone()).unwrap();
        assert!(room.remove_event(ObjectId::new()));
    }
    #[test]
    fn test_remove_event_normal() {
        let mut room = Room::default();
        let now = Local::now();
        let event = defaultevent(now, None);
        room.add_event(event.clone()).unwrap();
        assert!(room.remove_event(event.id));
        //assert!(room.events.is_empty());
    }
}
