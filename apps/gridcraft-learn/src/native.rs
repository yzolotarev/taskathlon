//! Native file dialogs for the learning app's embedded spreadsheet editor.
use gridcraft_ui_egui::Services;

pub fn services() -> Services {
    Services {
        pick_open: Some(Box::new(|| {
            rfd::FileDialog::new()
                .add_filter("Spreadsheets", &["xlsx", "xlsm", "xls", "xlsb", "ods", "csv", "tsv", "txt", "json"])
                .add_filter("Excel Workbook", &["xlsx", "xlsm"])
                .add_filter("Excel 97-2003 Workbook (import)", &["xls"])
                .add_filter("Excel Binary Workbook (data import)", &["xlsb"])
                .add_filter("OpenDocument Spreadsheet (data import)", &["ods"])
                .add_filter("CSV", &["csv"])
                .pick_file()
                .and_then(|path| path.to_str().map(str::to_string))
        })),
        pick_save: Some(Box::new(|suggested| {
            let path = std::path::Path::new(suggested);
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Book1");
            const TYPES: [(&str, &str); 4] =
                [("Excel Workbook (.xlsx)", "xlsx"), ("CSV UTF-8 (.csv)", "csv"), ("Web Page (.html)", "html"), ("PDF (.pdf)", "pdf")];
            let extension = path.extension().and_then(|s| s.to_str()).map(str::to_ascii_lowercase);
            let extension = extension.as_deref().filter(|ext| TYPES.iter().any(|(_, candidate)| candidate == ext)).unwrap_or("xlsx");
            let mut dialog = rfd::FileDialog::new().set_file_name(format!("{stem}.{extension}"));
            // Export dialogs suggest their own type; put that filter first.
            for (label, ext) in TYPES.iter().filter(|(_, ext)| *ext == extension).chain(TYPES.iter().filter(|(_, ext)| *ext != extension)) {
                dialog = dialog.add_filter(*label, &[*ext]);
            }
            dialog.save_file().and_then(|path| path.to_str().map(str::to_string))
        })),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn native_services_enable_file_menu_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let mut native = services();
        assert!(native.pick_open.is_some());
        assert!(native.pick_save.is_some());
        // Substitute only the OS picker, then exercise the real menu handlers and IO.
        let file = std::env::temp_dir().join(format!("taskathlon-native-dialog-{}.xlsx", std::process::id()));
        let save_path = file.to_string_lossy().into_owned();
        let open_path = save_path.clone();
        native.pick_save = Some(Box::new(move |_| Some(save_path.clone())));
        native.pick_open = Some(Box::new(move || Some(open_path.clone())));
        let mut app = gridcraft_learn::LearningApp::new(None)?;
        app.start_lesson(None)?;
        app.sheet.services = native;
        app.sheet.run("cell.set", json!({"cell": "A1", "value": 321}))?;
        app.sheet.open_dialog("saveAs", json!({}));
        assert!(file.is_file(), "Save As must call the registered picker and write the workbook");
        // File Open selects an already open file; close it first to verify the disk read.
        app.sheet.session.close_document(0);
        app.sheet.run("file.new", json!({}))?;
        app.sheet.run("cell.set", json!({"cell": "A1", "value": 999}))?;
        app.sheet.open_dialog("open", json!({}));
        let value = app.sheet.run("cell.get", json!({"cell": "A1"}))?;
        assert_eq!(value["value"].as_f64(), Some(321.0), "Open must load the selected workbook, replacing unsaved cell contents");
        std::fs::remove_file(file)?;
        Ok(())
    }
}
