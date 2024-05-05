use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
};

use crate::list::{TaskItem, TaskState};
use color_eyre::eyre::Result;

pub async fn load_tasks(file_path: &str) -> Result<Vec<TaskItem>> {
    if (fs::metadata(&file_path).await).is_err() {
        return Ok(vec![]);
    }

    let file = File::open(&file_path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut items = Vec::new();

    while let Some(line) = lines.next_line().await? {
        let state_char = match line.chars().nth(3) {
            Some(char) => char,
            None => continue,
        };

        let state = match state_char {
            'x' => TaskState::Done,
            _ => TaskState::Open,
        };

        let text = match line.split("] ").nth(1) {
            Some(text) => text.to_owned(),
            None => continue,
        };

        items.push(TaskItem::new(text, state))
    }

    Ok(items)
}

fn get_state_char(state: &TaskState) -> String {
    match state {
        TaskState::Done => "x".to_owned(),
        TaskState::Open => " ".to_owned(),
    }
}

pub async fn write_tasks(file_path: &str, tasks: Vec<TaskItem>) -> Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)
        .await?;

    let mut writer = BufWriter::new(file);

    let lines: Vec<String> = tasks
        .iter()
        .map(|task| format!("- [{}] {}", get_state_char(&task.state), task.text))
        .collect();

    for line in lines {
        writer.write_all(line.as_bytes()).await?;
        writer.write_all(b"\n").await?;
    }

    writer.flush().await?;

    Ok(())
}
