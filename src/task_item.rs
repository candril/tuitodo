#[derive(Clone)]
pub enum TaskState {
    Done,
    Open,
}

#[derive(Clone)]
pub struct TaskItem {
    pub state: TaskState,
    pub text: String,
}

impl TaskItem {
    pub fn new(text: String, state: TaskState) -> Self {
        Self { text, state }
    }

    pub fn toggle_state(&mut self) {
        self.state = match self.state {
            TaskState::Open => TaskState::Done,
            TaskState::Done => TaskState::Open,
        }
    }
}
