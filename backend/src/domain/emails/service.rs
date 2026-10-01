use serde_json::Value;

pub fn render(template: &str, payload: Option<&Value>) -> Option<(String, String)> {
    let name = payload
        .and_then(|value| value.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("there");
    let slot = payload
        .and_then(|value| value.get("slot_start"))
        .map(ToString::to_string)
        .unwrap_or_default();
    let timezone = payload
        .and_then(|value| value.get("timezone"))
        .and_then(Value::as_str)
        .unwrap_or("UTC");
    let result = match template {
        "booking_confirmation" => (
            "Your Cadence call is confirmed".to_owned(),
            format!("Hi {name},\n\nYour call is confirmed for {slot} ({timezone}).\n\nCadence"),
        ),
        "reminder_24h" => (
            "Reminder: your Cadence call is tomorrow".to_owned(),
            format!("Hi {name},\n\nYour Cadence call starts in 24 hours: {slot} ({timezone})."),
        ),
        "reminder_2h" => (
            "Reminder: your Cadence call starts in 2 hours".to_owned(),
            format!("Hi {name},\n\nYour Cadence call starts in 2 hours: {slot} ({timezone})."),
        ),
        "reminder_30min" => (
            "Reminder: your Cadence call starts in 30 minutes".to_owned(),
            format!("Hi {name},\n\nYour Cadence call starts in 30 minutes: {slot} ({timezone})."),
        ),
        _ => return None,
    };
    Some(result)
}
