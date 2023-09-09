use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Routine {
    pub name: String,
    pub events: Vec<(f32, Event)>,
}

impl Routine {
    pub fn merge(&mut self, other: &Self) {
        let mut events = other.events.clone();
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Event {
    Tongue {
        index: TongueIndex,
        speed: Option<f32>,
    },
    Constriction {
        i: usize,
        strength: Option<f32>,
    },
    Velum {
        openness: f32,
    },
    Pitch {
        value: f32,
    },
    Sound {
        sound: bool,
    },
    ForceDiameter,
}

impl Event {
    pub fn name(&self) -> &'static str {
        match self {
            Event::Tongue { .. } => "Tongue",
            Event::Constriction { .. } => "Constriction",
            Event::Velum { .. } => "Velum",
            Event::Pitch { .. } => "Pitch",
            Event::Sound { .. } => "Sound",
            Event::ForceDiameter => "Force Diameter",
        }
    }

    pub fn kind(&self) -> EventKind {
        match self {
            Event::Tongue { .. } => EventKind::Tongue,
            Event::Constriction { .. } => EventKind::Constriction,
            Event::Velum { .. } => EventKind::Velum,
            Event::Pitch { .. } => EventKind::Pitch,
            Event::Sound { .. } => EventKind::Sound,
            Event::ForceDiameter => EventKind::ForceDiameter,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TongueIndex {
    Index(usize),
    Random,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventKind {
    Tongue,
    Constriction,
    Velum,
    Pitch,
    Sound,
    ForceDiameter,
}

#[derive(Default)]
pub struct Runtime {
    events: Vec<(f32, Event)>,
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

    pub fn process(&mut self, dtime: f32, mut dispatch: impl FnMut(Event)) {
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
