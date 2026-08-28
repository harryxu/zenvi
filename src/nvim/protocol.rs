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
