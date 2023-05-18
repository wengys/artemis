use log::warn;
use plist::Dictionary;
use serde::Serialize;

use crate::artifacts::os::macos::plist::property_list::{get_dictionary, get_string};

#[derive(Debug, Serialize)]
pub(crate) struct Log {
    pub(crate) message: String,
    pub(crate) facility: String,
    pub(crate) log_level: String,
    pub(crate) log_type: String,
    pub(crate) parameters: Dictionary,
}

impl Log {
    /// Parse the Log Action `Emond` Rule
    pub(crate) fn parse_action_log(action_dictionary: &Dictionary) -> Log {
        let mut log_data = Log {
            message: String::new(),
            facility: String::new(),
            log_level: String::new(),
            log_type: String::new(),
            parameters: Dictionary::new(),
        };

        for (key, action_value) in action_dictionary {
            if key == "message" {
                log_data.message = get_string(action_value).unwrap_or_default();
            } else if key == "logLevel" {
                log_data.log_level = get_string(action_value).unwrap_or_default();
            } else if key == "logType" {
                log_data.log_type = get_string(action_value).unwrap_or_default();
            } else if key == "parameters" {
                log_data.parameters = get_dictionary(action_value).unwrap_or_default();
            } else if key == "facility" {
                log_data.facility = get_string(action_value).unwrap_or_default();
            } else if key == "type" {
                // Skip type values. We already know the action type
                continue;
            } else {
                warn!("[emond] Unknown Log Action key: {key}. Value: {action_value:?}");
            }
        }
        log_data
    }
}

#[cfg(test)]
mod tests {
    use plist::{Dictionary, Value};

    use crate::artifacts::os::macos::emond::actions::log::Log;

    #[test]
    fn test_parse_action_log() {
        let mut test_dictionary = Dictionary::new();
        test_dictionary.insert(String::from("message"), Value::String(String::from("test")));
        test_dictionary.insert(
            String::from("logLevel"),
            Value::String(String::from("level1")),
        );
        test_dictionary.insert(
            String::from("logType"),
            Value::String(String::from("type1")),
        );
        test_dictionary.insert(
            String::from("parameters"),
            Value::Dictionary(Dictionary::new()),
        );
        test_dictionary.insert(
            String::from("facility"),
            Value::String(String::from("testing")),
        );

        let results = Log::parse_action_log(&test_dictionary);
        assert_eq!(results.message, "test");
        assert_eq!(results.log_level, "level1");
        assert_eq!(results.log_type, "type1");
        assert_eq!(results.facility, "testing");
        assert_eq!(results.parameters, Dictionary::new());
    }
}
