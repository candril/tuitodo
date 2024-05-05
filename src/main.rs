mod file;
mod list;
mod tui;
use clap::Parser;
use core::panic;
use std::time::Duration;

use color_eyre::eyre::Result;
use crossterm::event::{
    self,
    KeyCode::{self, Char},
};
use file::{load_tasks, write_tasks};
use list::{TaskItem, TaskList};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::{self, UnboundedSender};
use tui::Event;
use tui_input::{backend::crossterm::EventHandler, Input};

// App state
struct App {
    file_path: String,
    counter: i64,
    should_quit: bool,
    action_tx: UnboundedSender<Action>,
    mode: Mode,
    new_task: Input,
    tasks: TaskList,
}

#[derive(PartialEq, Clone)]
pub enum Mode {
    Normal,
    Edit,
    Create,
}

// App actions
#[derive(Clone)]
pub enum Action {
    Tick,
    Increment,
    Decrement,
    NetworkRequestAndThenIncrement, // new
    NetworkRequestAndThenDecrement, // new
    Quit,
    Render,
    None,
    NextTask,
    PreviousTask,
    ToggleTaskState,
    HandleInputKey(event::Event),
    AddTask,
    ClearNewTask,
    SaveTask,
    SwitchMode(Mode),
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// file path of the todo file to load
    #[arg(short, long)]
    file: String,
}

fn ui(f: &mut Frame, app: &mut App) {
    let center = centered_rect(f.size(), 80, 30);

    let task_count = app.tasks.items.len() as u16;
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(task_count), Constraint::Length(1)])
        .split(center);

    list::ui(f, layout[0], &mut app.tasks);

    if app.mode != Mode::Create {
        return;
    }

    let input = Paragraph::new(app.new_task.value());

    let input_line = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Length(2), Constraint::Min(1)])
        .split(layout[1]);

    f.render_widget(Paragraph::new("\u{f460}"), input_line[0]);
    f.render_widget(input, input_line[1]);

    let width = layout[0].width.max(3) - 1;
    let scroll = app.new_task.visual_scroll(width as usize);

    f.set_cursor(
        layout[1].x + ((app.new_task.visual_cursor()).max(scroll) - scroll) as u16 + 2,
        layout[1].y,
    )
}

fn centered_rect(r: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn get_action(_app: &App, event: Event) -> Action {
    match event {
        Event::Error => Action::None,
        Event::Tick => Action::Tick,
        Event::Render => Action::Render,
        Event::Key(key, event) => match _app.mode {
            Mode::Normal => match key.code {
                Char('e') => Action::SwitchMode(Mode::Edit),
                Char('j') => Action::NextTask,
                Char('k') => Action::PreviousTask,
                KeyCode::Enter => Action::SwitchMode(Mode::Create),
                Char(' ') => Action::ToggleTaskState,
                Char('q') => Action::Quit,
                _ => Action::None,
            },
            Mode::Create => match key.code {
                KeyCode::Esc => Action::ClearNewTask,
                KeyCode::Enter => Action::AddTask,
                _ => Action::HandleInputKey(event),
            },
            Mode::Edit => match key.code {
                KeyCode::Esc => Action::ClearNewTask,
                KeyCode::Enter => Action::SaveTask,
                _ => Action::HandleInputKey(event),
            },
        },
        _ => Action::None,
    }
}

fn update(app: &mut App, action: Action) -> Option<Action> {
    match action {
        Action::Increment => {
            app.counter += 1;
        }
        Action::Decrement => {
            app.counter -= 1;
        }
        Action::NetworkRequestAndThenIncrement => {
            let tx = app.action_tx.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(5)).await; // simulate network request
                tx.send(Action::Increment).unwrap();
            });
        }
        Action::NetworkRequestAndThenDecrement => {
            let tx = app.action_tx.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(1)).await; // simulate network request
                tx.send(Action::Decrement).unwrap();
            });
        }

        Action::NextTask => {
            app.tasks.next();
        }
        Action::PreviousTask => {
            app.tasks.previous();
        }

        Action::ClearNewTask => {
            app.new_task.reset();
            app.mode = Mode::Normal
        }

        Action::AddTask => {
            app.tasks.items.push(TaskItem::new(
                app.new_task.value().into(),
                list::TaskState::Open,
            ));
            app.new_task.reset();

            let tasks = app.tasks.items.clone();
            let path = app.file_path.clone();
            tokio::spawn(async move { write_tasks(&path, tasks).await });
        }

        Action::HandleInputKey(event) => {
            app.new_task.handle_event(&event);
        }

        Action::ToggleTaskState => {
            if let Some(index) = app.tasks.state.selected() {
                app.tasks.items[index].toggle_state();

                let tasks = app.tasks.items.clone();
                let path = app.file_path.clone();
                tokio::spawn(async move { write_tasks(&path, tasks).await });
            }
        }

        Action::SwitchMode(mode) => {
            app.mode = mode;
        }

        Action::Quit => app.should_quit = true,
        _ => {}
    };

    None
}

async fn run() -> Result<()> {
    let args = Args::parse();

    let Ok(tasks) = load_tasks(args.file.as_str()).await else {
        panic!("could not load tasks")
    };

    let (action_tx, mut action_rx) = mpsc::unbounded_channel(); // new

    // ratatui terminal
    let mut tui = tui::Tui::new()?.tick_rate(1.0).frame_rate(30.0);
    tui.enter()?;

    let mut app = App {
        file_path: args.file,
        counter: 0,
        should_quit: false,
        action_tx: action_tx.clone(),
        new_task: Input::default(),
        mode: Mode::Normal,
        tasks: TaskList {
            state: ListState::default(),
            items: tasks,
        },
    };

    loop {
        let e = tui.next().await.unwrap();
        match e {
            tui::Event::Tick => action_tx.send(Action::Tick)?,
            tui::Event::Render => action_tx.send(Action::Render)?,
            tui::Event::Key(..) => {
                let action = get_action(&app, e);
                action_tx.send(action.clone())?;
            }
            _ => {}
        };

        while let Ok(action) = action_rx.try_recv() {
            let mut maybe_action = Some(action);

            while let Some(act) = maybe_action {
                let next_action = update(&mut app, act.clone());
                if let Action::Render = act {
                    tui.draw(|f| {
                        ui(f, &mut app);
                    })?;
                }
                maybe_action = next_action;
            }
        }

        if app.should_quit {
            break;
        }
    }

    tui.exit()?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let result = run().await;

    result?;

    Ok(())
}
