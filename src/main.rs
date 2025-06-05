use chrono::{DateTime, FixedOffset, Local, NaiveTime};
use clap::{Parser, Subcommand};
use regex::Regex;
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::PathBuf,
};

use umya_spreadsheet::{Spreadsheet, Worksheet, new_file, structs::Style, writer::xlsx::write};

#[derive(Parser)]
#[command(name = "attendance")]
#[command(about = "kintai: Attendance Record Manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Start,
    Finish {
        content: Option<String>,
    },
    BreakStart,
    BreakEnd,
    Summary {
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(short, long)]
        rate: Option<f64>,
    },
    Excel {
        #[arg(short, long)]
        input: Option<PathBuf>,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

struct LogEvent {
    ts: String,
    ty: String,
    content: Option<String>,
}

struct ActiveSession {
    start: DateTime<FixedOffset>,
    breaks: Vec<(DateTime<FixedOffset>, DateTime<FixedOffset>)>,
    last_break_start: Option<DateTime<FixedOffset>>,
}

struct Session {
    date: String,
    time_range: String,
    content: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Commands::Start => record_event("start", None)?,
        Commands::Finish { content } => record_event("finish", content)?,
        Commands::BreakStart => record_event("break_start", None)?,
        Commands::BreakEnd => record_event("break_end", None)?,
        Commands::Summary { input, rate } => {
            export_markdown(input.clone())?;
            summary_markdown(input, rate)?
        }
        Commands::Excel { input, output } => export_excel(input, output)?,
    }
    Ok(())
}

fn record_event(event_type: &str, content: Option<String>) -> anyhow::Result<()> {
    let ts = Local::now().to_rfc3339();
    let mut line = format!("ts={ts} type={event_type}");
    if let Some(c) = content {
        let esc = c.replace('"', "\\\"");
        line.push_str(&format!(" content=\"{esc}\""));
    }
    println!("{line}");
    Ok(())
}

fn build_sessions(mut events: Vec<LogEvent>) -> Vec<Session> {
    events.sort_by_key(|e| e.ts.clone());
    let mut sessions = Vec::new();
    let mut active: Option<ActiveSession> = None;

    for e in events {
        let dt = DateTime::parse_from_rfc3339(&e.ts).unwrap();
        match e.ty.as_str() {
            "start" => {
                active = Some(ActiveSession {
                    start: dt,
                    breaks: Vec::new(),
                    last_break_start: None,
                });
            }
            "break_start" => {
                if let Some(a) = active.as_mut() {
                    a.last_break_start = Some(dt);
                }
            }
            "break_end" => {
                if let Some(a) = active.as_mut() {
                    if let Some(bs) = a.last_break_start.take() {
                        a.breaks.push((bs, dt));
                    }
                }
            }
            "finish" => {
                if let Some(a) = active.take() {
                    let finish = dt;
                    let mut intervals = Vec::new();
                    let mut cursor = a.start;
                    for (bs, be) in &a.breaks {
                        intervals.push((cursor, *bs));
                        cursor = *be;
                    }
                    intervals.push((cursor, finish));
                    let parts: Vec<String> = intervals
                        .into_iter()
                        .map(|(s, e)| format!("{}~{}", s.format("%H:%M"), e.format("%H:%M")))
                        .collect();
                    sessions.push(Session {
                        date: a.start.format("%Y/%m/%d").to_string(),
                        time_range: parts.join(","),
                        content: e.content,
                    });
                }
            }
            _ => {}
        }
    }
    sessions
}

fn export_markdown(input: Option<PathBuf>) -> anyhow::Result<()> {
    let events = read_events(input)?;
    let sessions = build_sessions(events);
    println!("| date | time | content |");
    println!("|------|------|---------|");
    for s in sessions {
        println!(
            "| {} | {} | {} |",
            s.date,
            s.time_range,
            s.content.unwrap_or_default()
        );
    }
    println!();
    Ok(())
}

fn summary_markdown(input: Option<PathBuf>, rate: Option<f64>) -> anyhow::Result<()> {
    let events = read_events(input)?;
    let sessions = build_sessions(events);
    use std::collections::BTreeMap;
    let mut monthly: BTreeMap<String, f64> = BTreeMap::new();
    for s in &sessions {
        let month = &s.date[..7];
        let mut total = 0f64;
        for part in s.time_range.split(',') {
            let times: Vec<&str> = part.split('~').collect();
            if let [start, end] = &times[..] {
                let st = NaiveTime::parse_from_str(start, "%H:%M").unwrap();
                let en = NaiveTime::parse_from_str(end, "%H:%M").unwrap();
                total += (en - st).num_minutes() as f64 / 60.0;
            }
        }
        *monthly.entry(month.to_string()).or_default() += total;
    }
    let rate = rate.unwrap_or(0.0);
    println!("| month | hours | salary |");
    println!("|-------|-------|--------|");
    for (m, h) in monthly {
        let hours_i = h.floor() as u64;
        let mins = ((h - hours_i as f64) * 60.0).round() as u64;
        let dec_str = format!("{h:.2}h");
        let hours_str = format!("{hours_i}h{mins:02}m");
        let salary = (h * rate).round() as u64;
        println!("| {m} | {hours_str} ({dec_str}) | {salary} |");
    }
    println!();
    Ok(())
}

fn read_events(input: Option<PathBuf>) -> anyhow::Result<Vec<LogEvent>> {
    let reader: Box<dyn BufRead> = if let Some(path) = input {
        Box::new(BufReader::new(File::open(path)?))
    } else {
        Box::new(BufReader::new(io::stdin()))
    };

    let re =
        Regex::new(r#"ts=(?P<ts>[^ ]+) type=(?P<ty>[^ ]+)(?: content="(?P<ct>.*)")?"#).unwrap();
    let mut events: Vec<LogEvent> = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if let Some(caps) = re.captures(&line) {
            events.push(LogEvent {
                ts: caps["ts"].to_string(),
                ty: caps["ty"].to_string(),
                content: caps.name("ct").map(|m| m.as_str().to_string()),
            });
        }
    }
    Ok(events)
}

fn export_excel(input: Option<PathBuf>, output: Option<PathBuf>) -> anyhow::Result<()> {
    let events = read_events(input)?;
    let sessions = build_sessions(events);

    if sessions.is_empty() {
        println!("Log data is empty. Skipping Excel output.");
        return Ok(());
    }

    let first_date = &sessions[0].date;
    let first_ym = &first_date[..7];
    let parts: Vec<&str> = first_ym.split('/').collect();
    let year = parts[0];
    let month = parts[1];
    let title_text = format!("{year}年{}月の勤務時間記録", month.trim_start_matches('0'));

    let filtered: Vec<&Session> = sessions
        .iter()
        .filter(|s| s.date.starts_with(first_ym))
        .collect();

    let mut rows: Vec<(String, String, String)> = Vec::new();
    let mut total_minutes: i64 = 0;

    for s in &filtered {
        let parts: Vec<&str> = s.date.split('/').collect();
        let mm: &str = parts[1];
        let dd: &str = parts[2];
        let month_jp = format!(
            "{}月{}日",
            mm.trim_start_matches('0'),
            dd.trim_start_matches('0')
        );

        let time_str = s.time_range.clone();
        let content_str = s.content.clone().unwrap_or_default();

        for segment in s.time_range.split(',') {
            let times: Vec<&str> = segment.split('~').collect();
            if let [start, end] = &times[..] {
                let st = NaiveTime::parse_from_str(start, "%H:%M").unwrap();
                let en = NaiveTime::parse_from_str(end, "%H:%M").unwrap();
                total_minutes += (en - st).num_minutes();
            }
        }

        rows.push((month_jp, time_str, content_str));
    }

    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    let total_label = format!("{hours}時間{minutes}分");

    let mut max_b_len: usize = 0;
    for (_date, time_str, _content) in &rows {
        let len = time_str.chars().count(); // char 単位でカウント
        if len > max_b_len {
            max_b_len = len;
        }
    }
    let header_b_len = "勤務時間".chars().count();
    if header_b_len > max_b_len {
        max_b_len = header_b_len;
    }
    let mut book: Spreadsheet = new_file();

    let sheet_name = "Sheet1";
    let sheet: &mut Worksheet = book.get_sheet_by_name_mut(sheet_name).unwrap();

    let style = Style::default();

    let col_b = sheet.get_column_dimension_mut("B");
    col_b.set_width(max_b_len as f64);

    fn col_to_letter(mut col: u32) -> String {
        let mut s = String::new();
        while col > 0 {
            let rem = ((col - 1) % 26) as u8;
            s.push((b'A' + rem) as char);
            col = (col - 1) / 26;
        }
        s.chars().rev().collect()
    }

    fn coord(col: u32, row: u32) -> String {
        format!("{}{}", col_to_letter(col), row)
    }

    {
        let cell = coord(1, 1);
        let c = sheet.get_cell_mut(cell.clone());
        c.set_value(title_text.clone());
        c.set_style(style.clone());
    }

    {
        let headers = ["日付", "勤務時間", "作業内容"];
        for (i, &h) in headers.iter().enumerate() {
            let cell = coord((i as u32) + 1, 3);
            let c = sheet.get_cell_mut(cell.clone());
            c.set_value(h.to_string());
            c.set_style(style.clone());
        }
    }

    for (i, (date_jp, time_str, content_str)) in rows.iter().enumerate() {
        let excel_row = 4 + i as u32;
        let cell_date = coord(1, excel_row);
        sheet
            .get_cell_mut(cell_date.clone())
            .set_value(date_jp.clone());
        let cell_time = coord(2, excel_row);
        sheet
            .get_cell_mut(cell_time.clone())
            .set_value(time_str.clone());
        let cell_content = coord(3, excel_row);
        sheet
            .get_cell_mut(cell_content.clone())
            .set_value(content_str.clone());
    }

    let data_end_row = 3 + rows.len() as u32;
    let label_row = data_end_row + 2;
    let value_row = data_end_row + 3;

    {
        let cell_label = coord(1, label_row);
        sheet
            .get_cell_mut(cell_label.clone())
            .set_value("勤務時間の合計".to_string());
    }

    {
        let cell_total = coord(1, value_row);
        sheet
            .get_cell_mut(cell_total.clone())
            .set_value(total_label.clone());
    }

    let out_path = output.unwrap_or_else(|| PathBuf::from(format!("{year}_{month}_勤務時間.xlsx")));
    let path_str = out_path.as_os_str().to_string_lossy();
    write(&book, path_str.as_ref())?;

    println!("Generated Excel file: {}", out_path.display());
    Ok(())
}
