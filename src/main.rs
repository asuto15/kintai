use chrono::{DateTime, FixedOffset, Local};
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
        content: String,
    },
    BreakStart,
    BreakEnd,
    Export {
        #[arg(short, long)]
        input: Option<PathBuf>,
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
    content: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Commands::Start => record_event("start", None)?,
        Commands::Finish { content } => record_event("finish", Some(content))?,
        Commands::BreakStart => record_event("break_start", None)?,
        Commands::BreakEnd => record_event("break_end", None)?,
        Commands::Export { input } => export_markdown(input)?,
    }
    Ok(())
}

fn record_event(event_type: &str, content: Option<String>) -> anyhow::Result<()> {
    let ts = Local::now().to_rfc3339();
    let mut line = format!("ts={} type={}", ts, event_type);
    if let Some(c) = content {
        let esc = c.replace('"', "\\\"");
        line.push_str(&format!(" content=\"{}\"", esc));
    }
    println!("{}", line);
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
                    let content = e.content.unwrap_or_default();
                    let intervals = if a.breaks.is_empty() {
                        vec![format!(
                            "{}~{}",
                            a.start.format("%H:%M"),
                            finish.format("%H:%M")
                        )]
                    } else {
                        let mut parts = Vec::new();
                        let first_break = &a.breaks[0];
                        parts.push(format!(
                            "{}~{}",
                            a.start.format("%H:%M"),
                            first_break.0.format("%H:%M")
                        ));
                        for window in a.breaks.windows(2) {
                            parts.push(format!(
                                "{}~{}",
                                window[0].1.format("%H:%M"),
                                window[1].0.format("%H:%M")
                            ));
                        }
                        let last_break = a.breaks.last().unwrap();
                        parts.push(format!(
                            "{}~{}",
                            last_break.1.format("%H:%M"),
                            finish.format("%H:%M")
                        ));
                        parts
                    };
                    sessions.push(Session {
                        date: a.start.format("%Y/%m/%d").to_string(),
                        time_range: intervals.join(","),
                        content,
                    });
                }
            }
            _ => {}
        }
    }
    sessions
}

fn export_markdown(input: Option<PathBuf>) -> anyhow::Result<()> {
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

    let sessions = build_sessions(events);

    println!("| date | time | content |");
    println!("|------|------|---------|");
    for s in sessions {
        println!("| {} | {} | {} |", s.date, s.time_range, s.content);
    }
    println!();

    Ok(())
}
