#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum StatusCode {
    Success = 0,
    Error = 1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Status {
    pub code: StatusCode,
    pub reason: String,
}

impl Status {
    pub fn success() -> Self {
        Self {
            code: StatusCode::Success,
            reason: String::new(),
        }
    }

    pub fn error(reason: impl Into<String>) -> Self {
        Self {
            code: StatusCode::Error,
            reason: reason.into(),
        }
    }
}
