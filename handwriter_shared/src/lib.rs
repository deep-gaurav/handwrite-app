use serde::{Serialize,Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub text: String,
    pub style: Option<u32>,
    pub bias: Option<f32>,
    pub color: Option<String>,
    pub width: Option<u32>,
    pub status: TaskStatus,
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Waiting(u32),
    Completed(TaskCompleteTypes),
}

impl TaskStatus {
    /// Returns `true` if the task_status is [`Waiting`].
    pub fn is_waiting(&self) -> bool {
        matches!(self, Self::Waiting(..))
    }

    /// Returns `true` if the task_status is [`Completed`].
    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Completed(..))
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum TaskCompleteTypes {
    Success(SuccessResult),
    Failed(String),
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResult {
    pub url: String,
    pub svg: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HandParameters {
    pub text: String,
    pub style: Option<u32>,
    pub bias: Option<f32>,
    pub color: Option<String>,
    pub width: Option<u32>,
}