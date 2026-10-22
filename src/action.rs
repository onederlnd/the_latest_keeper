pub enum ActionKind {
    Autopsy,
    DigGrave,
}
pub enum ActionStatus {
    InProgress,
    Done,
    Cancelled,
}
pub struct TimedAction {
    pub kind: ActionKind,
    pub duration: f32,
    pub elapsed: f32,
    pub anchor: (f32, f32),
}

const MOVE_TOLERANCE: f32 = 12.0;

impl TimedAction {
    pub fn new(kind: ActionKind, duration: f32, anchor: (f32, f32)) -> Self {
        Self {
            kind,
            duration,
            elapsed: 0.0,
            anchor,
        }
    }
    // add dt if `held`, else Cancelled; also Cancelled if player moved off anchor
    pub fn tick(&mut self, dt: f32, held: bool, player_pos: (f32, f32)) -> ActionStatus {
        let dx = player_pos.0 - self.anchor.0;
        let dy = player_pos.1 - self.anchor.1;
        if !held || (dx * dx + dy * dy).sqrt() > MOVE_TOLERANCE {
            return ActionStatus::Cancelled;
        }
        self.elapsed += dt;
        if self.elapsed >= self.duration {
            ActionStatus::Done
        } else {
            ActionStatus::InProgress
        }
    }

    pub fn progress(&self) -> f32 {
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    pub fn label(&self) -> &'static str {
        match self.kind {
            ActionKind::Autopsy => "Examining the body...",
            ActionKind::DigGrave => "Digging the grave...",
        }
    }
}
