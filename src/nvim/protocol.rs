use rmpv::Value;

#[derive(Debug, Clone)]
pub enum RpcMessage {
    Request {
        msgid: u32,
        method: String,
        params: Vec<Value>,
    },
    Response {
        msgid: u32,
        error: Value,
        result: Value,
    },
    Notification {
        method: String,
        params: Vec<Value>,
    },
}

impl RpcMessage {
    pub fn parse(val: Value) -> Option<Self> {
        let arr = val.as_array()?;
        if arr.is_empty() {
            return None;
        }

        let msg_type = arr[0].as_u64()?;
        match msg_type {
            0 => {
                if arr.len() < 4 {
                    return None;
                }
                let msgid = arr[1].as_u64()? as u32;
                let method = arr[2].as_str()?.to_string();
                let params = arr[3].as_array()?.clone();
                Some(RpcMessage::Request {
                    msgid,
                    method,
                    params,
                })
            }
            1 => {
                if arr.len() < 4 {
                    return None;
                }
                let msgid = arr[1].as_u64()? as u32;
                let error = arr[2].clone();
                let result = arr[3].clone();
                Some(RpcMessage::Response {
                    msgid,
                    error,
                    result,
                })
            }
            2 => {
                if arr.len() < 3 {
                    return None;
                }
                let method = arr[2-1].as_str()?.to_string(); // index 1 is method
                let params = arr[2].as_array()?.clone();
                Some(RpcMessage::Notification { method, params })
            }
            _ => None,
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            RpcMessage::Request {
                msgid,
                method,
                params,
            } => Value::Array(vec![
                Value::from(0),
                Value::from(*msgid),
                Value::from(method.clone()),
                Value::Array(params.clone()),
            ]),
            RpcMessage::Response {
                msgid,
                error,
                result,
            } => Value::Array(vec![
                Value::from(1),
                Value::from(*msgid),
                error.clone(),
                result.clone(),
            ]),
            RpcMessage::Notification { method, params } => Value::Array(vec![
                Value::from(2),
                Value::from(method.clone()),
                Value::Array(params.clone()),
            ]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request() {
        let val = Value::Array(vec![
            Value::from(0),
            Value::from(42u64),
            Value::from("nvim_command"),
            Value::Array(vec![Value::from("echo 'hello'")]),
        ]);

        let msg = RpcMessage::parse(val).expect("Failed to parse request");
        match msg {
            RpcMessage::Request {
                msgid,
                method,
                params,
            } => {
                assert_eq!(msgid, 42);
                assert_eq!(method, "nvim_command");
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].as_str(), Some("echo 'hello'"));
            }
            _ => panic!("Expected Request message"),
        }
    }

    #[test]
    fn test_parse_response_success() {
        let val = Value::Array(vec![
            Value::from(1),
            Value::from(100u64),
            Value::Nil,
            Value::from("response_data"),
        ]);

        let msg = RpcMessage::parse(val).expect("Failed to parse response");
        match msg {
            RpcMessage::Response {
                msgid,
                error,
                result,
            } => {
                assert_eq!(msgid, 100);
                assert!(error.is_nil());
                assert_eq!(result.as_str(), Some("response_data"));
            }
            _ => panic!("Expected Response message"),
        }
    }

    #[test]
    fn test_parse_response_error() {
        let val = Value::Array(vec![
            Value::from(1),
            Value::from(101u64),
            Value::from("Syntax error in command"),
            Value::Nil,
        ]);

        let msg = RpcMessage::parse(val).expect("Failed to parse error response");
        match msg {
            RpcMessage::Response {
                msgid,
                error,
                result,
            } => {
                assert_eq!(msgid, 101);
                assert_eq!(error.as_str(), Some("Syntax error in command"));
                assert!(result.is_nil());
            }
            _ => panic!("Expected Response message with error"),
        }
    }

    #[test]
    fn test_parse_notification() {
        let val = Value::Array(vec![
            Value::from(2),
            Value::from("redraw"),
            Value::Array(vec![Value::Array(vec![
                Value::from("grid_clear"),
                Value::Array(vec![Value::from(1)]),
            ])]),
        ]);

        let msg = RpcMessage::parse(val).expect("Failed to parse notification");
        match msg {
            RpcMessage::Notification { method, params } => {
                assert_eq!(method, "redraw");
                assert_eq!(params.len(), 1);
            }
            _ => panic!("Expected Notification message"),
        }
    }

    #[test]
    fn test_parse_invalid_messages() {
        // Non-array value
        assert!(RpcMessage::parse(Value::from("not an array")).is_none());
        assert!(RpcMessage::parse(Value::from(123)).is_none());

        // Empty array
        assert!(RpcMessage::parse(Value::Array(vec![])).is_none());

        // Unknown message type (e.g. 3 or 99)
        assert!(RpcMessage::parse(Value::Array(vec![Value::from(99), Value::from(1)])).is_none());

        // Truncated request (type 0 needs 4 elements)
        assert!(RpcMessage::parse(Value::Array(vec![
            Value::from(0),
            Value::from(1),
            Value::from("test"),
        ]))
        .is_none());

        // Truncated response (type 1 needs 4 elements)
        assert!(RpcMessage::parse(Value::Array(vec![
            Value::from(1),
            Value::from(1),
            Value::Nil,
        ]))
        .is_none());

        // Truncated notification (type 2 needs 3 elements)
        assert!(RpcMessage::parse(Value::Array(vec![
            Value::from(2),
            Value::from("test"),
        ]))
        .is_none());
    }

    #[test]
    fn test_to_value_roundtrip() {
        // Request roundtrip
        let req = RpcMessage::Request {
            msgid: 5,
            method: "nvim_input".to_string(),
            params: vec![Value::from("<Esc>")],
        };
        let req_val = req.to_value();
        let parsed_req = RpcMessage::parse(req_val).expect("Roundtrip request failed");
        match parsed_req {
            RpcMessage::Request {
                msgid,
                method,
                params,
            } => {
                assert_eq!(msgid, 5);
                assert_eq!(method, "nvim_input");
                assert_eq!(params, vec![Value::from("<Esc>")]);
            }
            _ => panic!("Expected Request"),
        }

        // Response roundtrip
        let resp = RpcMessage::Response {
            msgid: 10,
            error: Value::Nil,
            result: Value::from(true),
        };
        let resp_val = resp.to_value();
        let parsed_resp = RpcMessage::parse(resp_val).expect("Roundtrip response failed");
        match parsed_resp {
            RpcMessage::Response {
                msgid,
                error,
                result,
            } => {
                assert_eq!(msgid, 10);
                assert!(error.is_nil());
                assert_eq!(result, Value::from(true));
            }
            _ => panic!("Expected Response"),
        }

        // Notification roundtrip
        let notif = RpcMessage::Notification {
            method: "redraw".to_string(),
            params: vec![Value::from(1), Value::from(2)],
        };
        let notif_val = notif.to_value();
        let parsed_notif = RpcMessage::parse(notif_val).expect("Roundtrip notification failed");
        match parsed_notif {
            RpcMessage::Notification { method, params } => {
                assert_eq!(method, "redraw");
                assert_eq!(params, vec![Value::from(1), Value::from(2)]);
            }
            _ => panic!("Expected Notification"),
        }
    }
}
