# JSON Log Viewer

This is a log viewer for "structured logs".  It currently parses a format where a log file is a set of log entries separated by a newline, each log entry is an ISO-8601 formatted timestamp followed by a JSON object (with no newlines).

Example
```json
2023-05-31T19:51:05.947Z {"level":"INFO","tag":"Main","message":"Hello, world!"}
2023-05-31T19:52:05.947Z {"level":"INFO","tag":"Main","message":"This is a structured log file."}
```

This project is written in Rust. It uses egui for the UI and the application is based on [https://github.com/emilk/eframe_template/].

## Architecture

TODO

## Contributing

TODO

## TODO
* Hotkeys
* Keyboard navigation (default focus + tab)
* One-click copy on everything
* Time scroll bar
* Filter
* Start maximized
* Follow file
* Asynchronous search result streaming
* Highlight matched search terms
* Fix status line height
* Toggle line numbers
* Customize columns
    * Add columns that can reference context fields
    * Change column order
* Custom color schemes (egui-stylist, others)
* Application & Tab Icons: https://crates.io/crates/egui-phosphor & https://phosphoricons.com/
* Remove WASM support, focus on desktop application
    * Easy to restore WASM in future from egui template.

## Known Issues
* window's "no tabs open" content does not show on startup because Tree:is_empty is not true after creation.  After creating and closing a tab it then returns true.
* Moving and re-docking tabs is not working correctly.
* After closing either the "Log" or "Context" tabs, an ID conflict occurs between the root tab (for the LogView) and the remaining open tab's close button in the DockArea below it.
* log view table does not extend to the bottom of the content area
* window flashing on startup
