use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{List, ListDirection, ListItem, ListState},
    Frame,
};

use crate::task_item::{TaskItem, TaskState};

pub struct TaskList {
    pub state: ListState,
}

impl TaskList {
    pub fn previous(&mut self, length: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    length - 1
                } else {
                    i - 1
                }
            }
            None => length - 1,
        };
        self.state.select(Some(i));
    }

    pub fn next(&mut self, length: usize) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= length - 1 {
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
    let state_char = match item.state {
        TaskState::Done => "\u{f058}",
        TaskState::Open => "\u{f05d}",
    };

    ListItem::from(format!("{} {}", state_char, item.text.clone()))
}

pub fn ui(f: &mut Frame, area: Rect, tasks: &[TaskItem], list_state: &mut ListState) {
    let items: Vec<ListItem> = tasks.iter().map(|i| item_ui(i)).collect();
    let list = List::new(items)
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_style(Style::default().fg(Color::Cyan))
        .direction(ListDirection::TopToBottom);

    f.render_stateful_widget(list, area, list_state);
}
