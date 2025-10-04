// EXACT 1:1 Translation from packages/types/src/ipc.ts
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Line 10-16: export enum IpcMessageType
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IpcMessageType {
    #[serde(rename = "Connect")]
    Connect,
    #[serde(rename = "Disconnect")]
    Disconnect,
    #[serde(rename = "Ack")]
    Ack,
    #[serde(rename = "TaskCommand")]
    TaskCommand,
    #[serde(rename = "TaskEvent")]
    TaskEvent,
}

// Line 22-25: export enum IpcOrigin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IpcOrigin {
    #[serde(rename = "client")]
    Client,
    #[serde(rename = "server")]
    Server,
}

// Line 31-37: export const ackSchema = z.object({...}) and export type Ack
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ack {
    pub client_id: String,
    pub pid: u32,
    pub ppid: u32,
}

// Line 43-48: export enum TaskCommandName
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskCommandName {
    #[serde(rename = "StartNewTask")]
    StartNewTask,
    #[serde(rename = "CancelTask")]
    CancelTask,
    #[serde(rename = "CloseTask")]
    CloseTask,
    #[serde(rename = "ResumeTask")]
    ResumeTask,
}

// Line 54-78: export const taskCommandSchema and export type TaskCommand
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "commandName")]
pub enum TaskCommand {
    StartNewTask {
        data: StartNewTaskData,
    },
    CancelTask {
        data: String,
    },
    CloseTask {
        data: String,
    },
    ResumeTask {
        data: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartNewTaskData {
    pub configuration: HashMap<String, serde_json::Value>,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "newTab")]
    pub new_tab: Option<bool>,
}

// Line 84-104: export const ipcMessageSchema and export type IpcMessage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IpcMessage {
    Ack {
        origin: IpcOrigin,
        data: Ack,
    },
    TaskCommand {
        origin: IpcOrigin,
        #[serde(rename = "clientId")]
        client_id: String,
        data: TaskCommand,
    },
    TaskEvent {
        origin: IpcOrigin,
        #[serde(rename = "relayClientId", skip_serializing_if = "Option::is_none")]
        relay_client_id: Option<String>,
        data: serde_json::Value, // taskEventSchema
    },
}

// Zod-like validation (matching ipcMessageSchema.safeParse)
pub struct SafeParseResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ParseError>,
}

pub struct ParseError {
    pub message: String,
}

impl ParseError {
    pub fn format(&self) -> String {
        self.message.clone()
    }
}

pub fn ipc_message_schema_safe_parse(data: &serde_json::Value) -> SafeParseResult<IpcMessage> {
    match serde_json::from_value::<IpcMessage>(data.clone()) {
        Ok(message) => SafeParseResult {
            success: true,
            data: Some(message),
            error: None,
        },
        Err(e) => SafeParseResult {
            success: false,
            data: None,
            error: Some(ParseError {
                message: e.to_string(),
            }),
        },
    }
}
