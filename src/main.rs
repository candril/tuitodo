mod list;
mod tui;

use std::time::Duration;

use color_eyre::eyre::Result;
use crossterm::event::KeyCode::{self, Char};
use list::TaskList;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::{self, UnboundedSender};
use tui::Event;

// App state
struct App<'a> {
    counter: i64,
    should_quit: bool,
    action_tx: UnboundedSender<Action>,

    tasks: TaskList<'a>,
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
    NextItem,
    PreviousItem,
    AddItem,
}

fn ui(f: &mut Frame, app: &mut App) {
    // let layout = Layout::default()
    //     .direction(Direction::Vertical)
    //     .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
    //     .split(f.size());

    let center = centered_rect(f.size(), 30, 30);
    list::ui(f, center, &mut app.tasks);
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
        Event::Key(key) => {
            match key.code {
                Char('j') => Action::NextItem,
                Char('k') => Action::PreviousItem,
                KeyCode::Enter => Action::AddItem,
                Char('J') => Action::NetworkRequestAndThenIncrement, // new
                Char('K') => Action::NetworkRequestAndThenDecrement, // new
                Char('q') => Action::Quit,
                _ => Action::None,
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

        Action::NextItem => {
            app.tasks.next();
        }
        Action::PreviousItem => {
            app.tasks.previous();
        }

        Action::AddItem => {
            app.tasks.items.push("Ziger");
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
            tui::Event::Key(_) => {
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
