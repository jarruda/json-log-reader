# JSON Log Viewer

This is a log viewer for "structured logs".  It currently parses a log format where the file is a set of log entries separated by a newline, each log entry is an ISO-8601 formatted timestamp followed by a JSON object (with no newlines). The file is expected to be UTF-8 encoded.

Example
```
2023-05-31T19:51:05.947Z {"level":"INFO","tag":"Main","message":"Hello, world!"}
2023-05-31T19:52:05.947Z {"level":"INFO","tag":"Main","message":"This is a structured log file."}
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
