use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[derive(Default)]
pub struct Routine {
    pub events: Vec<(f64, Event)>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Event {
    Tongue { i: usize, speed: Option<f64> },
    Constriction { i: usize, strength: f64 },
    Velum { openness: f64 },
    Pitch { value: f64 },
    Sound { sound: bool },
}

impl Event {
    pub fn kind(&self) -> EventKind {
        match self {
            Event::Tongue { .. } => EventKind::Tongue,
            Event::Constriction { .. } => EventKind::Constriction,
            Event::Velum { .. } => EventKind::Velum,
            Event::Pitch { .. } => EventKind::Pitch,
            Event::Sound { .. } => EventKind::Sound,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventKind {
    Tongue,
    Constriction,
    Velum,
    Pitch,
    Sound,
}

#[derive(Default)]
pub struct Runtime {
    events: Vec<(f64, Event)>,
}

impl Runtime {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn push_routine(&mut self, routine: &Routine) {
        // remove the same type events
        let mut kinds = routine
            .events
            .iter()
            .map(|(_, e)| e.kind())
            .collect::<Vec<_>>();
        kinds.sort();
        kinds.dedup();
        let mut i = 0;
        while i < self.events.len() {
            if kinds.contains(&self.events[i].1.kind()) {
                if i < self.events.len() - 1 {
                    self.events[i + 1].0 += self.events[i].0;
                }
                self.events.remove(i);
            } else {
                i += 1;
            }
        }

        let mut events = routine.events.clone();
        let mut merged = Vec::new();

        while !self.events.is_empty() && !events.is_empty() {
            if self.events[0].0 < events[0].0 {
                events[0].0 -= self.events[0].0;
                merged.push(self.events.remove(0));
            } else {
                self.events[0].0 -= events[0].0;
                merged.push(events.remove(0));
            }
        }

        merged.extend(self.events.drain(..));
        merged.extend(events.drain(..));

        self.events = merged;
    }

    pub fn process(&mut self, dtime: f64, mut dispatch: impl FnMut(Event)) {
        while !self.events.is_empty() {
            self.events[0].0 -= dtime;
            if self.events[0].0 > 0.0 {
                break;
            }
            let event = self.events.remove(0).1;
            dispatch(event);
        }
    }
}
