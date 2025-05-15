use chrono::{DateTime, FixedOffset, Local, NaiveTime};
use clap::{Parser, Subcommand};
use regex::Regex;
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::PathBuf,
};

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
        },
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
