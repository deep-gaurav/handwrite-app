use serde::{Serialize,Deserialize};
// use handwriter::strokes::Stroke;

#[derive(Clone, Debug, Serialize, Deserialize,PartialEq)]
pub struct Task {
    pub id: String,
    pub text: String,
    pub style: Option<u32>,
    pub bias: Option<f32>,
    pub color: Option<String>,
    pub width: Option<u32>,
    pub status: TaskStatus,
}

#[derive(Debug,Clone,Serialize,Deserialize,PartialEq)]
pub enum WorkerStatus{
    Available,
    Working(Task),
}

impl WorkerStatus {
    /// Returns `true` if the worker_status is [`Available`].
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    /// Returns `true` if the worker_status is [`Working`].
    pub fn is_working(&self) -> bool {
        matches!(self, Self::Working(..))
    }
}


#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Working(u32, u32),
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

    /// Returns `true` if the task_status is [`Working`].
    pub fn is_working(&self) -> bool {
        matches!(self, Self::Working(..))
    }

    pub fn as_completed(&self) -> Option<&TaskCompleteTypes> {
        if let Self::Completed(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum TaskCompleteTypes {
    Success(SuccessResult),
    Failed(String),
}

impl TaskCompleteTypes {
    pub fn as_success(&self) -> Option<&SuccessResult> {
        if let Self::Success(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResult {
    pub url: String,
    pub svg: String,
    // pub stroke:Stroke,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HandParameters {
    pub text: String,
    pub style: Option<u32>,
    pub bias: Option<f32>,
    pub color: Option<String>,
    pub width: Option<u32>,
}

