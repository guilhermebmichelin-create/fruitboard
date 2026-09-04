use serde::Serialize;

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppHealth {
    status: &'static str,
    runtime: &'static str,
    version: &'static str,
}

#[tauri::command]
fn get_app_health() -> AppHealth {
    AppHealth {
        status: "ok",
        runtime: "desktop",
        version: env!("CARGO_PKG_VERSION"),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_app_health])
        .run(tauri::generate_context!())
        .expect("failed to run the Fruitboard desktop application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn health_command_matches_the_client_contract() {
        let response = get_app_health();

        assert_eq!(response.status, "ok");
        assert_eq!(response.runtime, "desktop");
        assert_eq!(response.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(
            serde_json::to_value(response).expect("health response should serialize"),
            json!({
                "status": "ok",
                "runtime": "desktop",
                "version": env!("CARGO_PKG_VERSION"),
            })
        );
    }
}
