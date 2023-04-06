//! event.rs - a room reservation used by [crate::room::Room]
//!
//! Copyright 2023 by Ben Mattes Krusekamp <ben.krause05@gmail.com>
use chrono::Timelike;
use chrono::{DateTime, Local};
use serde::*;
use std::cmp::Ordering;
#[derive(Debug, Serialize, Deserialize, Clone)]

/// events tracks the time upto a minute
pub struct Event {
    booker_id: String,
    headline: String,
    description: Option<String>,
    start: DateTime<Local>,
    stop: DateTime<Local>,
    isdummy: bool,
}
impl Event {
    pub fn create(
        booker_id: String,
        headline: String,
        description: Option<String>,
        start: DateTime<Local>,
        stop: DateTime<Local>,
    ) -> Option<Self> {
        let start = start.with_nanosecond(0)?.with_second(0)?;
        let stop = stop.with_nanosecond(0)?.with_second(0)?;
        if start >= stop || booker_id.is_empty() || headline.is_empty() {
            return None;
        }
        Some(Event {
            booker_id,
            headline,
            description,
            start,
            stop,
            isdummy: false,
        })
    }
    /// creates a dummy used for comparing to event
    pub fn create_dummy(date: DateTime<Local>) -> Self {
        Event {
            booker_id: "".to_string(),
            headline: "".to_string(),
            description: None,
            start: date,
            stop: date,
            isdummy: true,
        }
    }

    pub fn start(&self) -> DateTime<Local> {
        self.start
    }
    pub fn stop(&self) -> DateTime<Local> {
        self.stop
    }
    pub fn isdummy(&self) -> bool {
        self.isdummy
    }
    /// event also overlaps if [self] is in [event] or vice versa
    pub fn overlaps_with(&self, event: &Event) -> bool {
        if self.datetime_is_in(&event.start) || self.datetime_is_in(&event.stop) {
            return true;
        }
        false
    }
    /// if the start of the one is the stop of the other, they partial overlap
    pub fn partial_overlaps_with(&self, event: &Event) -> bool {
        if self.start == event.stop || self.stop == event.start {
            return true;
        }
        false
    }
    /// being on the edge of is ignored
    pub fn datetime_is_in(&self, datetime: &DateTime<Local>) -> bool {
        if datetime > &self.start && datetime < &self.stop {
            return true;
        }
        false
    }
}
impl PartialEq for Event {
    /// the start and stop need to be the same for this to work
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start
    }
}
impl Eq for Event {}
impl PartialOrd for Event {
    /// it is logical error to compare if the [Event]s overlap with each other but are not equal
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.start.partial_cmp(&other.start)
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Days, Timelike};
    fn defaultevent(start: DateTime<Local>, stop: Option<DateTime<Local>>) -> Event {
        if let Some(stop) = stop {
            Event::create(
                String::from("test_booker_id"),
                String::from("test event"),
                None,
                start,
                stop,
            )
            .expect("event creation with stop time failed")
        } else {
            Event::create(
                String::from("test_booker_id"),
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
    /// just try to add an [Event] on a newly created room
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
        assert!(room
            .remove_event_datetime(DateTime::<Local>::MAX_UTC.into())
            .is_some());
    }
    #[test]
    #[should_panic]
    fn test_remove_event_unknown_id() {
        let mut room = Room::default();
        let now = Local::now();
        let event = defaultevent(now, None);
        room.add_event(event.clone()).unwrap();
        assert!(room
            .remove_event_datetime(DateTime::<Local>::MAX_UTC.into())
            .is_some());
    }
    #[test]
    fn test_remove_event_normal() {
        let mut room = Room::default();
        let now = Local::now();
        let event = defaultevent(now, None);
        room.add_event(event.clone()).unwrap();
        assert!(room.remove_event_datetime(event.start).is_some());
    }
}
