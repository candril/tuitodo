use tokio::{
    fs::{self, File},
    io::{AsyncBufReadExt, BufReader},
};

use crate::list::TaskItem;
use color_eyre::eyre::Result;

pub async fn load_tasks() -> Result<Vec<TaskItem>> {
    let path = "file.txt";

    if (fs::metadata(path).await).is_err() {
        return Ok(vec![]);
    }

    let file = File::open(path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut items = Vec::new();

    while let Some(line) = lines.next_line().await? {
        items.push(TaskItem::new(line))
    }

    Ok(items)
}
