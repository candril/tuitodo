mod list;
mod tui;

use std::time::Duration;

use color_eyre::eyre::Result;
use crossterm::event::{
    self,
    KeyCode::{self, Char},
};
use list::TaskList;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::{self, UnboundedSender};
use tui::Event;
use tui_input::{backend::crossterm::EventHandler, Input};

// App state
struct App {
    counter: i64,
    should_quit: bool,
    action_tx: UnboundedSender<Action>,
    mode: Mode,
    new_task: Input,
    tasks: TaskList,
}

enum Mode {
    Normal,
    Input,
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
    EnterInsertMode,
    HandleInputKey(event::Event),
    AddTask,
    ClearNewTask,
}

fn ui(f: &mut Frame, app: &mut App) {
    let center = centered_rect(f.size(), 30, 30);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Min(10), Constraint::Length(1)])
        .split(center);

    list::ui(f, layout[0], &mut app.tasks);
    let input = Paragraph::new(app.new_task.value());
    f.render_widget(input, layout[1]);

    let width = layout[0].width.max(3) - 1;
    let scroll = app.new_task.visual_scroll(width as usize);
    match app.mode {
        Mode::Normal => {}
        Mode::Input => {
            // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
            f.set_cursor(
                // Put cursor past the end of the input text
                layout[1].x + ((app.new_task.visual_cursor()).max(scroll) - scroll) as u16,
                // Move one line down, from the border to the input line
                layout[1].y,
            )
        }
    }
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
        Event::Key(key, event) => {
            match _app.mode {
                Mode::Normal => {
                    match key.code {
                        Char('j') => Action::NextTask,
                        Char('k') => Action::PreviousTask,
                        KeyCode::Enter => Action::EnterInsertMode,
                        Char('J') => Action::NetworkRequestAndThenIncrement, // new
                        Char('K') => Action::NetworkRequestAndThenDecrement, // new
                        Char('q') => Action::Quit,
                        _ => Action::None,
                    }
                }
                Mode::Input => match key.code {
                    KeyCode::Esc => Action::ClearNewTask,
                    KeyCode::Enter => Action::AddTask,
                    _ => Action::HandleInputKey(event),
                },
            }
        }
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

        Action::EnterInsertMode => app.mode = Mode::Input,

        Action::ClearNewTask => {
            app.new_task.reset();
            app.mode = Mode::Normal
        }

        Action::AddTask => {
            app.tasks.items.push(app.new_task.value().into());
            app.new_task.reset();
        }

        Action::HandleInputKey(event) => {
            app.new_task.handle_event(&event);
        }

        Action::Quit => app.should_quit = true,
        _ => {}
    };

    None
}

async fn run() -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::unbounded_channel(); // new

    // ratatui terminal
    let mut tui = tui::Tui::new()?.tick_rate(1.0).frame_rate(30.0);
    tui.enter()?;

    // application state
    let mut app = App {
        counter: 0,
        should_quit: false,
        action_tx: action_tx.clone(),
        new_task: Input::default(),
        mode: Mode::Normal,
        tasks: TaskList {
            state: ListState::default(),
            items: vec![],
        },
    };

    loop {
        let e = tui.next().await.unwrap();
        match e {
            tui::Event::Quit => action_tx.send(Action::Quit)?,
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

            // application update
            while let Some(act) = maybe_action {
                let next_action = update(&mut app, act.clone());
                // render only when we receive Action::Render
                if let Action::Render = act {
                    tui.draw(|f| {
                        ui(f, &mut app);
                    })?;
                }
                maybe_action = next_action;
            }
        }

        // application exit
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
