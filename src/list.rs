use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{List, ListDirection, ListItem, ListState},
    Frame,
};

pub struct TaskList {
    pub state: ListState,
    pub items: Vec<TaskItem>,
}

pub enum TaskState {
    Done,
    Open,
}

pub struct TaskItem {
    pub task_state: TaskState,
    pub text: String,
}

impl TaskItem {
    pub fn new(text: String) -> Self {
        Self {
            text,
            task_state: TaskState::Open,
        }
    }

    pub fn toggle_state(&mut self) {
        self.task_state = match self.task_state {
            TaskState::Open => TaskState::Done,
            TaskState::Done => TaskState::Open,
        }
    }
}

impl TaskList {
    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => self.items.len() - 1,
        };
        self.state.select(Some(i));
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            // None => self.last_selected.unwrap_or(0),
            None => 0,
        };
        self.state.select(Some(i));
    }
}

fn item_ui(item: &TaskItem) -> ListItem {
    let state_char = match item.task_state {
        TaskState::Done => "x",
        TaskState::Open => " ",
    };

    ListItem::from(format!("- [{}] {}", state_char, item.text.clone()))
}

pub fn ui(f: &mut Frame, area: Rect, tasks: &mut TaskList) {
    let items: Vec<ListItem> = tasks.items.iter().map(|i| item_ui(i)).collect();

    let list = List::new(items)
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_style(Style::default().fg(Color::Cyan))
        .direction(ListDirection::TopToBottom);

    f.render_stateful_widget(list, area, &mut tasks.state);
}
