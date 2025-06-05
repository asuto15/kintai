# `kintai`: Attendance Record Manager

A simple CLI tool written in Rust for recording work sessions and breaks in **logfmt** format, then exporting detailed daily logs and monthly summaries to Markdown or Excel.

**Note:** The name `kintai` means "attendance" in Japanese.

## Features

- **Start** a work session, **Finish** with optional notes.
- Mark **Break Start** and **Break End**.
- **Summary** daily sessions as a Markdown table and monthly totals with salary calculation.
- **Excel**: Export a single-month attendance record to an `.xlsx` file (writes everything into `Sheet1`, automatically detects the month from the log).

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

### Generate Report (Markdown)

Use the `summary` command to produce both the daily session table and monthly summary:

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

### Export to Excel

Use the `excel` command to export a single month’s attendance into an Excel file (`.xlsx`). It automatically detects which month to export by looking at the first session’s date in the log. All data is written into **Sheet1**.

```sh
$ kintai excel --input work.log --output attendance_2025_04.xlsx
```

- If you omit `--input`, it reads from standard input:
  ```sh
  $ cat work.log | kintai excel --output attendance.xlsx
  ```
- If you omit `--output`, the default filename is `YYYY_MM_勤務時間.xlsx`, where `YYYY` and `MM` are automatically determined from the year and month of the first session recorded in the log.

Once run, you’ll see a message like:

```plaintext
Generated Excel file: attendance_2025_04.xlsx
```

Open that file and you’ll find:

- **Sheet1**:
  1. `A1`: Title (for example, `2025年4月の勤務時間記録`)
  2. Row 3: Header row with `日付 | 勤務時間 | 作業内容`
  3. From row 4 onward: Each session for that month (for example, `4月19日 | 15:30~16:30 | オンボーディング作業`, etc.)
  4. Below the table, insert a blank row, then include the labels `勤務時間の合計` and the total time (for example, `15時間9分`)

## Commands

- `start`
  Record the start timestamp of a session.

- `finish [--content <note>]`
  Record the end timestamp. Optionally add a note.

- `break-start` / `break-end`
  Mark beginning and end of a break.

- `summary [-i <file>] [-r <rate>]`
  Output daily sessions and monthly summary (reads from `<file>` or stdin, default rate = 0).

- `excel [-i <file>] [-o <file>]`
  Export one month’s attendance to Excel.
  - `-i, --input <file>`: Path to the log file (defaults to stdin if omitted).
  - `-o, --output <file>`: Path to the output `.xlsx` file (defaults to an auto-generated filename(`YYYY_MM_勤務時間.xlsx`) if omitted).


## Log Format

Each event is emitted in logfmt (key=value) on one line:

```ini
ts=2025-04-21T09:00:00+09:00 type=start
ts=2025-04-21T12:00:00+09:00 type=break_start
ts=2025-04-21T13:00:00+09:00 type=break_end
ts=2025-04-21T18:00:00+09:00 type=finish content="Project meeting"
```
