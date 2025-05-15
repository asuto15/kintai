# `kintai`: Attendance Record Manager

A simple CLI tool written in Rust for recording work sessions and breaks in **logfmt** format, then exporting detailed daily logs and monthly summaries to Markdown.

**Note:** The name `kintai` means "attendance" in Japanese.

## Features

- **Start** a work session, **Finish** with optional notes.
- Mark **Break Start** and **Break End**.
- **Summary** daily sessions as a Markdown table and monthly totals with salary calculation.

## Installation

1. Ensure you have Rust and Cargo installed.
2. Clone this repository and build:

   ```sh
   $ git clone <repo-url>
   $ cd kintai
   $ cargo build --release
   ```

3. Optionally install to your Cargo bin directory:

   ```sh
   $ cargo install --path .
   ```

## Usage

Record events by appending the output to a log file:

```sh
# Start a session
$ kintai start >> work.log

# Start a break
$ kintai break-start >> work.log

# End a break
$ kintai break-end >> work.log

# Finish with optional note
$ kintai finish -- "Project meeting" >> work.log
```

### Generate Report

Use the summary command to produce both the daily session table and monthly summary:

```sh
$ kintai summary --input work.log --rate 35.0
```

Example output:

```plaintext
| date       | time               | content         |
|------------|--------------------|-----------------|
| 2025/04/21 | 09:00~12:00,13:00~18:00 | Project meeting |

| month   | hours            | salary |
|---------|------------------|--------|
| 2025/04 | 8h00m (8.00h)    | 280    |
```

### Commands

- start
  Record the start timestamp of a session.

- finish [--content <note>]
  Record the end timestamp. Optionally add a note.

- break-start / break-end
  Mark beginning and end of a break.

- summary [-i <file>] [-r <rate>]Output daily sessions and monthly summary (reads from <file> or stdin, default rate = 0).

## Log Format

Each event is emitted in logfmt (key=value) on one line:

```ini
ts=2025-04-21T09:00:00+09:00 type=start
ts=2025-04-21T12:00:00+09:00 type=break_start
ts=2025-04-21T13:00:00+09:00 type=break_end
ts=2025-04-21T18:00:00+09:00 type=finish content="Project meeting"
```
