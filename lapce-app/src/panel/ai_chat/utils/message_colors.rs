// Message color utilities - ported from utils/messageColors.ts

pub fn get_message_color(message_type: &str) -> &'static str {
    match message_type {
        "error" => "editor.errorForeground",
        "warning" => "editor.warnForeground",
        "success" => "editor.infoForeground",
        _ => "editor.foreground",
    }
}
