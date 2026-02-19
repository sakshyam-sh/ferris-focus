use crate::models::{SessionType, FOCUS_DURATION_SECS, LONG_BREAK_SECS, SHORT_BREAK_SECS, SESSIONS_BEFORE_LONG_BREAK};

/// timer state
#[derive(Debug, Clone)]
pub enum TimerState {
    Idle,
    Running {
        remaining_secs: u32,
        session_type: SessionType,
    },
    Paused {
        remaining_secs: u32,
        session_type: SessionType,
    },
    Finished {
        session_type: SessionType,
    },
}

/// timer engine, pure state machine
#[derive(Debug, Clone)]
pub struct Timer {
    pub state: TimerState,
    pub focus_sessions_completed: u32,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            state: TimerState::Idle,
            focus_sessions_completed: 0,
        }
    }

    /// start session of given type
    pub fn start(&mut self, session_type: SessionType) {
        let duration = match session_type {
            SessionType::Focus => FOCUS_DURATION_SECS,
            SessionType::ShortBreak => SHORT_BREAK_SECS,
            SessionType::LongBreak => LONG_BREAK_SECS,
        };
        self.state = TimerState::Running {
            remaining_secs: duration,
            session_type,
        };
    }

    /// auto-start next session
    pub fn start_next(&mut self) {
        let next_type = self.next_session_type();
        self.start(next_type);
    }

    /// next session type to run
    pub fn next_session_type(&self) -> SessionType {
        match &self.state {
            TimerState::Finished { session_type } => match session_type {
                SessionType::Focus => {
                    if self.focus_sessions_completed > 0
                        && self.focus_sessions_completed % SESSIONS_BEFORE_LONG_BREAK == 0
                    {
                        SessionType::LongBreak
                    } else {
                        SessionType::ShortBreak
                    }
                }
                SessionType::ShortBreak | SessionType::LongBreak => SessionType::Focus,
            },
            _ => SessionType::Focus,
        }
    }

    /// tick 1s, returns true if finished
    pub fn tick(&mut self) -> bool {
        if let TimerState::Running {
            remaining_secs,
            session_type,
        } = &self.state
        {
            let session_type = *session_type;
            if *remaining_secs <= 1 {
                if session_type == SessionType::Focus {
                    self.focus_sessions_completed += 1;
                }
                self.state = TimerState::Finished { session_type };
                return true;
            }
            self.state = TimerState::Running {
                remaining_secs: remaining_secs - 1,
                session_type,
            };
        }
        false
    }

    /// pause if running
    pub fn pause(&mut self) {
        if let TimerState::Running {
            remaining_secs,
            session_type,
        } = &self.state
        {
            self.state = TimerState::Paused {
                remaining_secs: *remaining_secs,
                session_type: *session_type,
            };
        }
    }

    /// resume if paused
    pub fn resume(&mut self) {
        if let TimerState::Paused {
            remaining_secs,
            session_type,
        } = &self.state
        {
            self.state = TimerState::Running {
                remaining_secs: *remaining_secs,
                session_type: *session_type,
            };
        }
    }

    /// reset to idle
    pub fn reset(&mut self) {
        self.state = TimerState::Idle;
    }

    /// remaining as (min, sec)
    pub fn remaining_display(&self) -> (u32, u32) {
        let secs = match &self.state {
            TimerState::Running { remaining_secs, .. } => *remaining_secs,
            TimerState::Paused { remaining_secs, .. } => *remaining_secs,
            _ => 0,
        };
        (secs / 60, secs % 60)
    }

    /// total session duration in secs
    pub fn total_duration_secs(&self) -> u32 {
        let session_type = match &self.state {
            TimerState::Running { session_type, .. } => Some(session_type),
            TimerState::Paused { session_type, .. } => Some(session_type),
            _ => None,
        };
        match session_type {
            Some(SessionType::Focus) => FOCUS_DURATION_SECS,
            Some(SessionType::ShortBreak) => SHORT_BREAK_SECS,
            Some(SessionType::LongBreak) => LONG_BREAK_SECS,
            None => FOCUS_DURATION_SECS,
        }
    }

    /// elapsed fraction 0.0..1.0
    pub fn progress(&self) -> f32 {
        let remaining = match &self.state {
            TimerState::Running { remaining_secs, .. } => *remaining_secs,
            TimerState::Paused { remaining_secs, .. } => *remaining_secs,
            TimerState::Finished { .. } => 0,
            TimerState::Idle => return 0.0,
        };
        let total = self.total_duration_secs();
        if total == 0 {
            return 0.0;
        }
        1.0 - (remaining as f32 / total as f32)
    }

    /// current session type if any
    pub fn current_session_type(&self) -> Option<SessionType> {
        match &self.state {
            TimerState::Running { session_type, .. } => Some(*session_type),
            TimerState::Paused { session_type, .. } => Some(*session_type),
            TimerState::Finished { session_type } => Some(*session_type),
            TimerState::Idle => None,
        }
    }

    /// is running?
    pub fn is_running(&self) -> bool {
        matches!(self.state, TimerState::Running { .. })
    }

    /// is paused?
    pub fn is_paused(&self) -> bool {
        matches!(self.state, TimerState::Paused { .. })
    }

    /// is finished?
    pub fn is_finished(&self) -> bool {
        matches!(self.state, TimerState::Finished { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_decrements() {
        let mut timer = Timer::new();
        timer.start(SessionType::Focus);
        let finished = timer.tick();
        assert!(!finished);
        let (m, s) = timer.remaining_display();
        assert_eq!(m * 60 + s, FOCUS_DURATION_SECS - 1);
    }

    #[test]
    fn test_timer_finishes() {
        let mut timer = Timer::new();
        timer.state = TimerState::Running {
            remaining_secs: 1,
            session_type: SessionType::Focus,
        };
        let finished = timer.tick();
        assert!(finished);
        assert!(timer.is_finished());
    }

    #[test]
    fn test_pause_resume() {
        let mut timer = Timer::new();
        timer.start(SessionType::Focus);
        timer.tick(); // 24:59
        timer.pause();
        assert!(timer.is_paused());
        let (m, s) = timer.remaining_display();
        timer.resume();
        assert!(timer.is_running());
        let (m2, s2) = timer.remaining_display();
        assert_eq!((m, s), (m2, s2));
    }

    #[test]
    fn test_long_break_after_4() {
        let mut timer = Timer::new();
        timer.focus_sessions_completed = 4;
        timer.state = TimerState::Finished {
            session_type: SessionType::Focus,
        };
        assert_eq!(timer.next_session_type(), SessionType::LongBreak);
    }

    #[test]
    fn test_short_break_after_focus() {
        let mut timer = Timer::new();
        timer.focus_sessions_completed = 1;
        timer.state = TimerState::Finished {
            session_type: SessionType::Focus,
        };
        assert_eq!(timer.next_session_type(), SessionType::ShortBreak);
    }

    #[test]
    fn test_focus_after_break() {
        let mut timer = Timer::new();
        timer.state = TimerState::Finished {
            session_type: SessionType::ShortBreak,
        };
        assert_eq!(timer.next_session_type(), SessionType::Focus);
    }

    #[test]
    fn test_progress() {
        let mut timer = Timer::new();
        timer.start(SessionType::Focus);
        assert!((timer.progress() - 0.0).abs() < f32::EPSILON);

        // halfway
        timer.state = TimerState::Running {
            remaining_secs: FOCUS_DURATION_SECS / 2,
            session_type: SessionType::Focus,
        };
        assert!((timer.progress() - 0.5).abs() < 0.01);
    }
}
