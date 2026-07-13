use chew_domain::Cadre;
use serde::Serialize;

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeContract {
    pub product_name: &'static str,
    pub offline: bool,
    pub supported_cadres: [&'static str; 2],
}

pub fn runtime_contract_value() -> RuntimeContract {
    let supported_cadres = Cadre::ALL.map(|cadre| match cadre {
        Cadre::Jchew => "JCHEW",
        Cadre::Chew => "CHEW",
    });

    RuntimeContract {
        product_name: "CHEW Companion",
        offline: true,
        supported_cadres,
    }
}

#[tauri::command]
fn runtime_contract() -> RuntimeContract {
    runtime_contract_value()
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![runtime_contract])
        .run(tauri::generate_context!())
        .expect("failed to run CHEW Companion");
}

#[cfg(test)]
mod tests {
    use super::runtime_contract_value;

    #[test]
    fn runtime_contract_is_offline_and_supports_both_cadres() {
        let contract = runtime_contract_value();
        assert_eq!(contract.product_name, "CHEW Companion");
        assert!(contract.offline);
        assert_eq!(contract.supported_cadres, ["JCHEW", "CHEW"]);
    }
}
