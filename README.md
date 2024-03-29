# JSON Log Viewer
[![Rust](https://github.com/jarruda/json-log-reader/actions/workflows/rust.yml/badge.svg)](https://github.com/jarruda/json-log-reader/actions/workflows/rust.yml)

This is a log viewer for "structured logs".  It currently parses a log format where the file is a set of log entries separated by a newline, each log entry is a JSON object (with no newlines). The file is expected to be UTF-8 encoded.

The current expected mandatory log entries are:
* t: string; The ISO-8601-formatted entry timestamp.
* level: string; The log entry's severity in the set {DEBUG, INFO, WARNING, ERROR, FATAL}
* tag: string; An arbitrary string for the entry.
* message: string; The entry's message.

Entries can contain any other number of key/value pairs that will be displayed in the "Context" tab when a log entry is selected.

Example
```
{"t": "2023-05-31T19:51:05.947Z", level":"INFO","tag":"Main","message":"Hello, world!"}
{"t": "2023-05-31T19:52:05.947Z", level":"INFO","tag":"Main","message":"This is a structured log file."}
```

This project is written in Rust. It uses egui for the UI and the application is based on https://github.com/emilk/eframe_template.

## Architecture

TODO

## Contributing

TODO

## TODO
* Follow file (tail)
* Hotkeys
* Keyboard navigation
    * Default focus + tabbing
    * Up/Down line selection
    * Page Up/Down scrolling on tables
* One-click copy on everything
* Start maximized
* Highlight matched search terms
* Fix status line height
* Toggle line number display
* Customize columns
    * Add columns that can reference context fields
    * Change column order
* Customize fonts, color scheme, etc (egui-stylist, others)
* Time scroll bar
    * Show a time representation next to the scroll bar a la Google Photos
* Asynchronous search result streaming
* Asynchronous file loading (newline counting)
* Customize file format (timestamp + implicit JSON keys for message, tag, and level)
* Application & Tab Icons: https://crates.io/crates/egui-phosphor & https://phosphoricons.com/
* Plugin support: Load shared libraries dynamically to gain additional functionality.
* Log statistics: tab reporting number of lines, log size, duration of period included, log entry count per severity, etc.
* CloudWatch Logs: open a log stream like a file
* Cloudwatch Logs Insights: make insights queries and view results as a log

## Known Issues
* Moving and re-docking tabs is not working correctly - possibly because nested edock::Tree's are used, unknown if supported.
* After closing either the "Log" or "Context" tabs, an ID conflict occurs between the root tab (for the LogView) and the remaining open tab's close button in the DockArea below it.
* The main window's "no tabs open" content does not show on startup because `Tree::is_empty` is not true after creation.  After creating and closing a tab it then returns true.
* Window flashes on startup before displaying final window
