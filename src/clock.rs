// src/clock.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Weekday {
    Moon,
    Tyr,
    Woden,
    Thor,
    Freyja,
    Sun,
}

const WEEK: [Weekday; 6] = [
    Weekday::Moon,
    Weekday::Tyr,
    Weekday::Woden,
    Weekday::Thor,
    Weekday::Freyja,
    Weekday::Sun,
];

impl Weekday {
    fn from_index(i: u32) -> Weekday {
        WEEK[(i as usize) % WEEK.len()]
    }
}

pub struct GameClock {
    pub day_index: u32,
    pub time_of_day: f32,
    pub day_length_secs: f32,
}

impl GameClock {
    pub fn new() -> Self {
        GameClock {
            day_index: 0,
            time_of_day: 0.0,
            day_length_secs: 450.0,
        }
    }
    pub fn weekday(&self) -> Weekday {
        Weekday::from_index(self.day_index)
    }
    pub fn tick(&mut self, dt: f32) {
        self.time_of_day += dt / self.day_length_secs;
        while self.time_of_day >= 1.0 {
            self.time_of_day -= 1.0;
            self.day_index += 1;
        }
    }
    pub fn is_between(&self, start: f32, end: f32) -> bool {
        if start <= end {
            self.time_of_day >= start && self.time_of_day < end
        } else {
            self.time_of_day >= start || self.time_of_day < end
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ticking_past_one_day_rolls_over() {
        let mut clock = GameClock::new();
        clock.day_length_secs = 10.0;
        clock.tick(9.0);
        assert_eq!(clock.day_index, 0);
        clock.tick(2.0);
        assert_eq!(clock.day_index, 1);
        assert!(clock.time_of_day < 0.2);
    }

    #[test]
    fn weekday_cycles_after_six_days() {
        let mut clock = GameClock::new();
        clock.day_index = 6;
        assert_eq!(clock.weekday(), Weekday::Moon)
    }

    #[test]
    fn is_between_handles_wraparround_window() {
        let mut clock = GameClock::new();
        clock.time_of_day = 0.95;
        assert!(clock.is_between(0.9, 0.1));
        clock.time_of_day = 0.5;
        assert!(!clock.is_between(0.9, 0.1));
    }
}
