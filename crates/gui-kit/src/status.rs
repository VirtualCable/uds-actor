/*!
Author: Andres Schumann, aschumann at virtualcable dot es
*/
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Ready,
    Busy,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub text: String,
    pub level: Level,
}

impl Status {
    pub fn new(level: Level, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level,
        }
    }

    pub fn is_busy(&self) -> bool {
        self.level == Level::Busy
    }
}

impl Default for Status {
    fn default() -> Self {
        Self::new(Level::Ready, "Ready")
    }
}
